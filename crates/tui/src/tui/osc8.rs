//! OSC 8 hyperlink emission and stripping.
//!
//! Modern terminals (iTerm2, Terminal.app 13+, Ghostty, Kitty, WezTerm,
//! Alacritty, recent gnome-terminal/konsole) make a substring clickable when
//! it is wrapped in:
//!
//! ```text
//! \x1b]8;;TARGET\x1b\\LABEL\x1b]8;;\x1b\\
//! ```
//!
//! Terminals that don't understand the sequence simply render the visible
//! `LABEL` and ignore the escape. So emitting OSC 8 is a strict UX upgrade for
//! supporting terminals and a no-op for the rest.
//!
//! ratatui 0.30.1 filters control characters from `Span` content
//! (via `set_stringn`), so OSC 8 bytes can no longer be embedded directly.
//! Instead we accumulate link registrations during rendering and inject
//! OSC 8 sequences into the buffer cells post-render, using
//! `Cell::set_symbol` (which does not filter) and `CellDiffOption::ForcedWidth`
//! to keep the diff width aligned with the visible label.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};

use ratatui::buffer::{Buffer, CellDiffOption};

const OSC8_PREFIX: &str = "\x1b]8;;";
const OSC8_TERMINATOR: &str = "\x1b\\";

/// Process-wide enable flag. `true` by default. Set once at app init from
/// `[ui] osc8_links` (when present) and read by the renderer.
static ENABLED: AtomicBool = AtomicBool::new(true);

/// Set the process-wide OSC 8 enable flag. Intended to be called once at
/// startup; subsequent calls take effect immediately.
pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

/// Whether OSC 8 hyperlink emission is currently enabled.
#[must_use]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

thread_local! {
    static LINK_REGISTRY: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
}

/// Register a link label→URL mapping. Called during widget rendering.
/// The label is the visible text; the URL is the link target.
fn register_link(label: String, url: String) {
    LINK_REGISTRY.with(|r| r.borrow_mut().push((label, url)));
}

/// Take accumulated link registrations and clear for the next frame.
pub fn take_links() -> Vec<(String, String)> {
    LINK_REGISTRY.with(|r| r.take())
}

/// Return the visible label for a link, registering the label→URL mapping
/// so `apply_links` can inject OSC 8 into the buffer post-render.
/// When OSC 8 is disabled, returns the label as-is without registering.
#[must_use]
pub fn wrap_link_label(target: &str, label: &str) -> String {
    if enabled() {
        register_link(label.to_string(), target.to_string());
    }
    label.to_string()
}

/// Legacy wrapper kept for callers that still embed OSC 8 in spans.
/// When OSC 8 is enabled, now just returns the label (escape codes are
/// injected post-render). When disabled, returns the label as-is.
#[must_use]
pub fn wrap_link(target: &str, label: &str) -> String {
    wrap_link_label(target, label)
}

/// Post-render step: scan the buffer for registered link labels and inject
/// OSC 8 escape sequences directly into cells via `Cell::set_symbol`.
/// Must be called after all widgets have rendered and before the frame
/// is sent to the terminal.
pub fn apply_links(buf: &mut Buffer) {
    let links = take_links();
    if links.is_empty() || !enabled() {
        return;
    }

    let area = *buf.area();
    for y in area.top()..area.bottom() {
        let mut x = area.left();
        while x < area.right() {
            'next_cell: for (label, url) in &links {
                let label_chars: Vec<char> = label.chars().collect();
                let label_len = label_chars.len();
                if label_len == 0 || x + label_len as u16 > area.right() {
                    continue;
                }

                // Check if the label matches this buffer position
                let mut matched = true;
                for (i, ch) in label_chars.iter().enumerate() {
                    let cell_symbol = buf[(x + i as u16, y)].symbol();
                    if cell_symbol != ch.to_string() {
                        matched = false;
                        break;
                    }
                }
                if !matched {
                    continue;
                }

                // Found a match — inject OSC 8 wrapping
                // First cell: OSC8_PREFIX + URL + OSC8_TERMINATOR + first char of label
                let mut first = String::with_capacity(
                    OSC8_PREFIX.len() + url.len() + OSC8_TERMINATOR.len() + 4,
                );
                first.push_str(OSC8_PREFIX);
                first.push_str(url);
                first.push_str(OSC8_TERMINATOR);
                if let Some(&first_ch) = label_chars.first() {
                    first.push(first_ch);
                }
                let first_cell = &mut buf[(x, y)];
                first_cell.set_symbol(&first);
                first_cell.set_diff_option(CellDiffOption::ForcedWidth(
                    std::num::NonZero::new(1).unwrap(),
                ));

                // Last cell: last char of label + OSC8_PREFIX + OSC8_TERMINATOR (close link)
                if label_len >= 2 {
                    let last_x = x + (label_len as u16) - 1;
                    let mut last =
                        String::with_capacity(OSC8_PREFIX.len() + OSC8_TERMINATOR.len() + 4);
                    if let Some(&last_ch) = label_chars.last() {
                        last.push(last_ch);
                    }
                    last.push_str(OSC8_PREFIX);
                    last.push_str(OSC8_TERMINATOR);
                    let last_cell = &mut buf[(last_x, y)];
                    last_cell.set_symbol(&last);
                    last_cell.set_diff_option(CellDiffOption::ForcedWidth(
                        std::num::NonZero::new(1).unwrap(),
                    ));
                } else {
                    // Single-char label: first cell already handles it, but also needs closing
                    let mut first_close = buf[(x, y)].symbol().to_string();
                    first_close.push_str(OSC8_PREFIX);
                    first_close.push_str(OSC8_TERMINATOR);
                    buf[(x, y)].set_symbol(&first_close);
                    buf[(x, y)].set_diff_option(CellDiffOption::ForcedWidth(
                        std::num::NonZero::new(1).unwrap(),
                    ));
                }

                // Advance past this label
                x += label_len as u16;
                continue 'next_cell;
            }
            x += 1;
        }
    }
}

/// Strip every ANSI escape sequence from `s` into `out`, preserving only the
/// visible characters. ratatui's buffer drops the leading `ESC` byte but
/// happily paints every other byte of an escape (`[`, `0`, `;`, `m`, OSC
/// payloads, etc.) into a buffer cell, drifting columns. Tool stdout that
/// includes ANSI (e.g. `gh`/`git` with color forced on, anything run through
/// a PTY) must be sanitized before it enters the transcript.
///
/// Handles CSI (`ESC [ … final`), OSC (`ESC ] … BEL` or `ESC \`), DCS, SOS,
/// PM, APC, and standalone two-byte ESC sequences. OSC 8 hyperlink wrappers
/// (`ESC ] 8 ; … BEL` / `ESC \`) are stripped along with the rest.
pub fn strip_ansi_into(s: &str, out: &mut String) {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            match next {
                // CSI: ESC [ ... <final byte 0x40..=0x7E>
                b'[' => {
                    let mut j = i + 2;
                    while j < bytes.len() {
                        let b = bytes[j];
                        if (0x40..=0x7e).contains(&b) {
                            j += 1;
                            break;
                        }
                        j += 1;
                    }
                    i = j;
                    continue;
                }
                // OSC / DCS / SOS / PM / APC: ESC ] | P | X | ^ | _ ... ST(ESC \) or BEL
                b']' | b'P' | b'X' | b'^' | b'_' => {
                    let mut j = i + 2;
                    while j < bytes.len() {
                        if bytes[j] == 0x07 {
                            j += 1;
                            break;
                        }
                        if bytes[j] == 0x1b && j + 1 < bytes.len() && bytes[j + 1] == b'\\' {
                            j += 2;
                            break;
                        }
                        j += 1;
                    }
                    i = j;
                    continue;
                }
                // Standalone two-byte ESC sequence (RIS, charset selection, etc.)
                _ => {
                    i += 2;
                    continue;
                }
            }
        }
        // Strip lone control bytes that ratatui would otherwise drop (and which
        // mean nothing in transcript output) but keep \n, \r, \t as legitimate
        // formatting.
        let b = bytes[i];
        if b < 0x80 {
            if b < 0x20 && b != b'\n' && b != b'\r' && b != b'\t' {
                i += 1;
                continue;
            }
            out.push(b as char);
            i += 1;
        } else {
            // UTF-8 multi-byte sequence: copy the whole code point intact.
            let len = utf8_seq_len(b);
            let end = (i + len).min(bytes.len());
            if let Ok(chunk) = std::str::from_utf8(&bytes[i..end]) {
                out.push_str(chunk);
            }
            i = end;
        }
    }
}

/// Length in bytes of the UTF-8 sequence that starts with `lead`. Falls back
/// to `1` for continuation bytes / invalid leads so callers always make
/// forward progress.
fn utf8_seq_len(lead: u8) -> usize {
    if lead < 0xc0 {
        1
    } else if lead < 0xe0 {
        2
    } else if lead < 0xf0 {
        3
    } else {
        4
    }
}

/// Strip OSC 8 escape sequences from `s` into `out`, preserving the visible
/// label text. Other escapes (color, style) pass through untouched. The
/// implementation handles both the standard `ESC \` and the lone `BEL`
/// terminators that some emitters use.
pub fn strip_into(s: &str, out: &mut String) {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Look for the OSC 8 prefix `ESC ] 8 ;`
        if i + 4 <= bytes.len()
            && bytes[i] == 0x1b
            && bytes[i + 1] == b']'
            && bytes[i + 2] == b'8'
            && bytes[i + 3] == b';'
        {
            // Skip until the string terminator (ESC \) or BEL.
            let mut j = i + 4;
            while j < bytes.len() {
                if bytes[j] == 0x07 {
                    j += 1;
                    break;
                }
                if bytes[j] == 0x1b && j + 1 < bytes.len() && bytes[j + 1] == b'\\' {
                    j += 2;
                    break;
                }
                j += 1;
            }
            i = j;
            continue;
        }
        let b = bytes[i];
        if b < 0x80 {
            out.push(b as char);
            i += 1;
        } else {
            let len = utf8_seq_len(b);
            let end = (i + len).min(bytes.len());
            if let Ok(chunk) = std::str::from_utf8(&bytes[i..end]) {
                out.push_str(chunk);
            }
            i = end;
        }
    }
}

/// Convenience: strip OSC 8 and return a new String.
#[must_use]
#[allow(dead_code)]
pub fn strip(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    strip_into(s, &mut out);
    out
}

/// Convenience: strip all ANSI sequences and return a new String.
#[must_use]
#[allow(dead_code)]
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    strip_ansi_into(s, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_link_shape_is_osc_8_compliant() {
        let wrapped = wrap_link("https://example.com", "click me");
        // With ratatui 0.30.1, wrap_link returns just the label (no escape codes).
        assert_eq!(wrapped, "click me");
    }

    #[test]
    fn strip_removes_wrapper_keeps_label() {
        let wrapped = wrap_link("https://example.com", "click me");
        assert_eq!(strip(&wrapped), "click me");
    }

    #[test]
    fn strip_preserves_ansi_that_is_not_osc_8() {
        let text = "\x1b[31mred\x1b[0m";
        assert_eq!(strip(text), text);
    }

    #[test]
    fn strip_removes_osc_8_inside_ansi_wrapped_text() {
        let wrapped = wrap_link("https://example.com", "click");
        let mixed = format!("\x1b[31mred\x1b[0m {wrapped}",);
        assert_eq!(strip(&mixed), "\x1b[31mred\x1b[0m click");
    }

    #[test]
    fn strip_does_not_touch_plain_text() {
        assert_eq!(strip("hello world"), "hello world");
    }

    #[test]
    fn strip_empty_is_empty() {
        assert_eq!(strip(""), "");
    }

    #[test]
    fn strip_handles_non_ascii_urls() {
        let wrapped = wrap_link("https://例子.com", "点击这里");
        assert_eq!(strip(&wrapped), "点击这里");
    }

    #[test]
    fn strip_ansi_removes_osc_8_wrapper() {
        let wrapped = wrap_link("https://example.com", "click");
        assert_eq!(strip_ansi(&wrapped), "click");
    }

    #[test]
    fn strip_ansi_removes_color_codes() {
        assert_eq!(
            strip_ansi("\x1b[31mred\x1b[0m and \x1b[32mgreen\x1b[0m"),
            "red and green"
        );
    }

    #[test]
    fn strip_ansi_preserves_unicode() {
        assert_eq!(strip_ansi("普\x1b[31m通\x1b[0m话"), "普通话");
    }

    #[test]
    fn strip_preserves_utf8_multibyte_chars() {
        let wrapped = wrap_link("https://example.com", "点击我");
        assert_eq!(strip(&wrapped), "点击我");
    }

    #[test]
    fn link_registry_accumulates_and_clears() {
        // Initial state: empty
        assert!(take_links().is_empty());

        // Register links
        let _ = wrap_link_label("https://a.com", "link A");
        let _ = wrap_link_label("https://b.com", "link B");

        let links = take_links();
        assert_eq!(links.len(), 2);
        assert_eq!(
            links[0],
            ("link A".to_string(), "https://a.com".to_string())
        );
        assert_eq!(
            links[1],
            ("link B".to_string(), "https://b.com".to_string())
        );

        // Should be empty after take
        assert!(take_links().is_empty());
    }
}
