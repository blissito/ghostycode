//! Expand a shell command line into the set of commands a shell would run.
//!
//! Deny rules are the one gate that holds under `AskForApproval::Never`, so
//! they cannot be matched against the raw command string: the string a user
//! types and the set of commands the shell executes are different things. A
//! command substitution runs its body (`` `rm -rf /` ``, `$(rm -rf /)`), a
//! quoted argument executes with the quotes removed (`rm -rf "/"`), and a
//! wrapper hands its payload straight back to a shell (`bash -c '…'`,
//! `eval '…'`, `sudo …`).
//!
//! Matching one string pattern per metacharacter loses that race by
//! construction — every new quoting or wrapping form is another bypass. This
//! module instead tokenizes the command the way a POSIX shell word-splits it
//! and returns *every* command line that would actually be executed, so deny
//! rules can be matched against each one.
//!
//! Deliberately conservative in the deny direction: when a construct is
//! ambiguous the expander emits extra candidate command lines rather than
//! fewer. Over-emitting only makes deny matching stricter — `denied_prefix_matches`
//! stays anchored at the first positional token, so an extra candidate that no
//! rule names is inert. Under-emitting is a bypass.
//!
//! What it does *not* do is evaluate anything: `$VAR` is left as literal text,
//! and single-quoted text is never treated as code (`echo '` + "`" + `rm -rf /`" +
//! "`" + `'` really does just print). Fidelity to shell semantics is the point in
//! both directions.

use std::collections::HashSet;

/// Maximum nesting depth followed through substitutions and `-c` payloads.
const MAX_DEPTH: usize = 8;

/// Upper bound on emitted command lines, so a pathological input cannot turn
/// one policy check into unbounded work.
const MAX_COMMANDS: usize = 256;

/// How far into a command the search for a wrapper head (`bash -c`, `eval`)
/// will walk past flags and wrapper words.
const MAX_HEAD_SCAN: usize = 8;

/// Words that prefix another command rather than being the command: the real
/// invocation is what follows. Stripping them keeps `sudo rm -rf /` matchable
/// by an `rm -rf /` rule.
const PASSTHROUGH_WRAPPERS: &[&str] = &[
    "sudo", "doas", "env", "nohup", "nice", "ionice", "time", "timeout", "stdbuf", "setsid",
    "command", "builtin", "exec", "xargs", "unbuffer", "busybox", "chroot", "proot",
];

/// Shells whose `-c` argument is a command line to be parsed, not an operand.
const SHELL_NAMES: &[&str] = &[
    "sh", "bash", "zsh", "dash", "ksh", "ksh93", "mksh", "ash", "fish", "csh", "tcsh", "rbash",
    "yash",
];

/// Returns every command line the shell would execute for `command`.
///
/// The raw input is always included first, so callers keep whatever matching
/// they already did against it. Subsequent entries are the word-split, quote-
/// stripped command lines drawn from top-level chaining, command substitutions,
/// process substitutions, grouping, and wrapper payloads. Results are
/// de-duplicated and order-stable.
pub fn expanded_commands(command: &str) -> Vec<String> {
    let mut expander = Expander {
        out: Vec::new(),
        seen: HashSet::new(),
    };
    let trimmed = command.trim();
    if !trimmed.is_empty() {
        expander.seen.insert(trimmed.to_string());
        expander.out.push(trimmed.to_string());
    }
    expander.expand(command, 0);
    expander.out
}

struct Expander {
    out: Vec<String>,
    seen: HashSet<String>,
}

impl Expander {
    fn emit(&mut self, tokens: &[String]) {
        if self.out.len() >= MAX_COMMANDS {
            return;
        }
        let joined = tokens
            .iter()
            .filter(|token| !token.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        if joined.is_empty() {
            return;
        }
        if self.seen.insert(joined.clone()) {
            self.out.push(joined);
        }
    }

    /// Word-split `input` into command lines and record each one, recursing
    /// into every nested command text found along the way.
    fn expand(&mut self, input: &str, depth: usize) {
        if depth > MAX_DEPTH || self.out.len() >= MAX_COMMANDS {
            return;
        }
        let chars: Vec<char> = input.chars().collect();
        let n = chars.len();
        let mut i = 0usize;
        let mut commands: Vec<Vec<String>> = Vec::new();
        let mut words: Vec<String> = Vec::new();
        let mut word = String::new();
        let mut started = false;
        let mut nested: Vec<String> = Vec::new();

        while i < n {
            let c = chars[i];
            match c {
                // A backslash outside quotes escapes exactly one character,
                // including an operator: `echo a\;b` is one word, not two
                // commands. A backslash-newline is a line continuation.
                '\\' => {
                    if i + 1 < n {
                        if chars[i + 1] != '\n' {
                            word.push(chars[i + 1]);
                            started = true;
                        }
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                // Single quotes are fully literal: no substitution, no escapes.
                '\'' => {
                    started = true;
                    i += 1;
                    while i < n && chars[i] != '\'' {
                        word.push(chars[i]);
                        i += 1;
                    }
                    i = (i + 1).min(n);
                }
                // Double quotes suppress word splitting but NOT substitution.
                '"' => {
                    started = true;
                    i += 1;
                    while i < n && chars[i] != '"' {
                        match chars[i] {
                            '\\' if i + 1 < n => {
                                word.push(chars[i + 1]);
                                i += 2;
                            }
                            '`' => {
                                let (inner, next) = read_backtick(&chars, i);
                                nested.push(inner);
                                i = next;
                            }
                            '$' if i + 1 < n && chars[i + 1] == '(' => {
                                let (inner, next) = read_delimited(&chars, i + 1, '(', ')');
                                nested.push(inner);
                                i = next;
                            }
                            '$' if i + 1 < n && chars[i + 1] == '{' => {
                                let (inner, next) = read_delimited(&chars, i + 1, '{', '}');
                                nested.push(inner);
                                i = next;
                            }
                            ch => {
                                word.push(ch);
                                i += 1;
                            }
                        }
                    }
                    i = (i + 1).min(n);
                }
                // `$'…'` (ANSI-C quoting) is literal text with C escapes.
                '$' if i + 1 < n && chars[i + 1] == '\'' => {
                    started = true;
                    i += 2;
                    while i < n && chars[i] != '\'' {
                        if chars[i] == '\\' && i + 1 < n {
                            word.push(chars[i + 1]);
                            i += 2;
                        } else {
                            word.push(chars[i]);
                            i += 1;
                        }
                    }
                    i = (i + 1).min(n);
                }
                // Command substitution, both spellings. The body is a command
                // line in its own right; the substitution contributes no text
                // to the enclosing word (we do not evaluate output).
                '`' => {
                    let (inner, next) = read_backtick(&chars, i);
                    nested.push(inner);
                    i = next;
                }
                '$' if i + 1 < n && chars[i + 1] == '(' => {
                    let (inner, next) = read_delimited(&chars, i + 1, '(', ')');
                    nested.push(inner);
                    i = next;
                }
                // `${…}` is an expansion, not a command — but it can *contain*
                // one (`${x:-$(rm -rf /)}`), so the body is rescanned.
                '$' if i + 1 < n && chars[i + 1] == '{' => {
                    let (inner, next) = read_delimited(&chars, i + 1, '{', '}');
                    nested.push(inner);
                    i = next;
                }
                // Process substitution `<(…)` / `>(…)` also runs its body.
                '<' | '>' if i + 1 < n && chars[i + 1] == '(' => {
                    let (inner, next) = read_delimited(&chars, i + 1, '(', ')');
                    nested.push(inner);
                    i = next;
                }
                ' ' | '\t' => {
                    flush_word(&mut words, &mut word, &mut started);
                    i += 1;
                }
                // A subshell boundary. `$(`, `<(` and `>(` were consumed by the
                // arms above, so a bare paren here is grouping: the body is a
                // command list of its own, not part of the surrounding word.
                '(' | ')' => {
                    flush_word(&mut words, &mut word, &mut started);
                    end_command(&mut commands, &mut words);
                    i += 1;
                }
                // Control operators end the current command line. `&&`, `||`,
                // `;;`, `|&` and runs of newlines collapse into one break.
                '\n' | '\r' | ';' | '&' | '|' => {
                    flush_word(&mut words, &mut word, &mut started);
                    end_command(&mut commands, &mut words);
                    i += 1;
                    while i < n && matches!(chars[i], '\n' | '\r' | ';' | '&' | '|') {
                        i += 1;
                    }
                }
                _ => {
                    word.push(c);
                    started = true;
                    i += 1;
                }
            }
        }
        flush_word(&mut words, &mut word, &mut started);
        end_command(&mut commands, &mut words);

        for tokens in &commands {
            self.record(tokens, depth);
        }
        for inner in nested {
            self.expand(&inner, depth + 1);
        }
    }

    /// Record one word-split command line, plus the invocation hiding inside it
    /// when the head is a wrapper.
    fn record(&mut self, tokens: &[String], depth: usize) {
        if tokens.is_empty() {
            return;
        }
        self.emit(tokens);

        // `sudo rm -rf /` is an `rm -rf /`. Strip wrapper words (and the scalar
        // arguments that belong to them, e.g. `timeout 5`) and emit what's left.
        let stripped = strip_leading_wrappers(tokens);
        if stripped.len() != tokens.len() {
            self.emit(stripped);
        }

        // `eval …` and `sh -c …` take a *command line* as data. Parse it.
        if let Some(head) = find_wrapper_head(tokens) {
            let name = basename(&tokens[head]).to_ascii_lowercase();
            if name == "eval" {
                let payload = tokens[head + 1..].join(" ");
                self.expand(&payload, depth + 1);
            } else if let Some(script) = shell_c_argument(&tokens[head..]) {
                self.expand(script, depth + 1);
            }
        }
    }
}

fn flush_word(words: &mut Vec<String>, word: &mut String, started: &mut bool) {
    if *started || !word.is_empty() {
        words.push(std::mem::take(word));
        *started = false;
    }
}

fn end_command(commands: &mut Vec<Vec<String>>, words: &mut Vec<String>) {
    // `{` and `}` stand alone as reserved words in `{ cmd; }` — they group a
    // command rather than being part of one. Dropping them here keeps every
    // downstream consumer (wrapper detection, emission) looking at real
    // command words only.
    words.retain(|word| !matches!(word.as_str(), "{" | "}"));
    if !words.is_empty() {
        commands.push(std::mem::take(words));
    }
}

/// Read a backtick substitution. `start` indexes the opening backtick; returns
/// the body and the index just past the closing backtick.
fn read_backtick(chars: &[char], start: usize) -> (String, usize) {
    let mut i = start + 1;
    let mut inner = String::new();
    while i < chars.len() {
        match chars[i] {
            '\\' if i + 1 < chars.len() => {
                inner.push(chars[i]);
                inner.push(chars[i + 1]);
                i += 2;
            }
            '`' => return (inner, i + 1),
            c => {
                inner.push(c);
                i += 1;
            }
        }
    }
    (inner, i)
}

/// Read a balanced `open`/`close` region. `open_at` indexes the opening
/// delimiter; returns the body and the index just past the matching close.
fn read_delimited(chars: &[char], open_at: usize, open: char, close: char) -> (String, usize) {
    let mut depth = 1usize;
    let mut i = open_at + 1;
    let mut inner = String::new();
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            inner.push(c);
            inner.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return (inner, i + 1);
            }
        }
        inner.push(c);
        i += 1;
    }
    (inner, i)
}

/// The final path component, so `/usr/bin/sudo` reads as `sudo`.
fn basename(token: &str) -> &str {
    token
        .rsplit(['/', '\\'])
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(token)
}

fn is_env_assignment(token: &str) -> bool {
    match token.split_once('=') {
        Some((name, _)) => {
            !name.is_empty()
                && !name.starts_with('-')
                && name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        }
        None => false,
    }
}

/// True for a bare scalar operand that belongs to a wrapper word rather than
/// starting a command — `timeout 5`, `nice -n 10`, `timeout 1.5s`.
fn is_scalar_operand(token: &str) -> bool {
    let body = token.trim_end_matches(['s', 'm', 'h', 'd']);
    !body.is_empty() && body.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
}

fn is_passthrough_wrapper(token: &str) -> bool {
    let name = basename(token).to_ascii_lowercase();
    PASSTHROUGH_WRAPPERS.contains(&name.as_str())
}

fn is_shell_name(token: &str) -> bool {
    let name = basename(token).to_ascii_lowercase();
    SHELL_NAMES.contains(&name.as_str())
}

/// Drop leading environment assignments, wrapper words, and the scalar operands
/// those wrappers take, returning the remaining slice.
///
/// Flags are deliberately *not* dropped: `denied_prefix_matches` already skips
/// unrelated flags (and, ambiguously, their values) when anchoring a rule, so
/// leaving `-u root` in place is both correct and matchable.
fn strip_leading_wrappers(tokens: &[String]) -> &[String] {
    let mut start = 0usize;
    let mut dropped_wrapper = false;
    while start < tokens.len() {
        let token = &tokens[start];
        if is_env_assignment(token) {
            start += 1;
        } else if is_passthrough_wrapper(token) {
            dropped_wrapper = true;
            start += 1;
        } else if dropped_wrapper && is_scalar_operand(token) {
            start += 1;
        } else {
            break;
        }
    }
    &tokens[start..]
}

/// Index of the `eval` / shell word that introduces a nested command line, if
/// this invocation has one.
///
/// The scan walks past environment assignments, wrapper words, flags, and the
/// operand immediately following a single-dash flag (which may be that flag's
/// value, as in `sudo -u root bash -c …`). It stops at the first token that
/// cannot plausibly precede the real command, which is what keeps
/// `echo bash -c 'rm -rf /'` — a command that only prints — from being read as
/// a shell invocation.
fn find_wrapper_head(tokens: &[String]) -> Option<usize> {
    let mut previous_was_short_flag = false;
    for (index, token) in tokens.iter().enumerate().take(MAX_HEAD_SCAN) {
        if is_shell_name(token) || basename(token).eq_ignore_ascii_case("eval") {
            return Some(index);
        }
        let skippable = is_env_assignment(token)
            || is_passthrough_wrapper(token)
            || token.starts_with('-')
            || is_scalar_operand(token)
            || previous_was_short_flag;
        if !skippable {
            return None;
        }
        previous_was_short_flag = token.starts_with('-') && !token.starts_with("--");
    }
    None
}

/// The command-line argument of a shell's `-c` flag, if present.
///
/// `tokens[0]` is the shell. Combined short flags count (`bash -lc '…'`).
/// The scan deliberately does NOT stop at the first non-flag operand: an
/// earlier version did, and `bash -o vi -c 'payload'` walked straight past
/// the deny expander because `vi` (the argument of `-o`) ended the scan
/// before `-c` was seen (2026-08-04 review). Continuing the scan can
/// over-read a `-c` that is really an argument to a script
/// (`bash script.sh -c x`), but this expander's contract is explicit that
/// over-emitting targets is safe and under-emitting is a bypass.
fn shell_c_argument(tokens: &[String]) -> Option<&str> {
    let mut index = 1usize;
    while index < tokens.len() {
        let token = tokens[index].as_str();
        let takes_command_line = match token.strip_prefix("--") {
            Some(long) => long.eq_ignore_ascii_case("command"),
            None => token
                .strip_prefix('-')
                .is_some_and(|flags| flags.contains('c')),
        };
        if takes_command_line {
            return tokens.get(index + 1).map(String::as_str);
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand(command: &str) -> Vec<String> {
        expanded_commands(command)
    }

    fn contains(command: &str, expected: &str) -> bool {
        expand(command).iter().any(|target| target == expected)
    }

    #[test]
    fn backtick_body_is_a_command() {
        assert!(contains("`rm -rf /`", "rm -rf /"));
        assert!(contains("echo `rm -rf /`", "rm -rf /"));
        assert!(contains("echo `rm -rf /`", "echo"));
    }

    #[test]
    fn dollar_paren_body_is_a_command() {
        assert!(contains("echo $(rm -rf /)", "rm -rf /"));
        assert!(contains("x=$(rm -rf /)", "rm -rf /"));
        assert!(contains("echo \"$(rm -rf /)\"", "rm -rf /"));
    }

    #[test]
    fn nested_substitution_is_followed() {
        assert!(contains("echo $(echo `rm -rf /`)", "rm -rf /"));
    }

    #[test]
    fn quotes_are_removed_from_operands() {
        assert!(contains("rm -rf \"/\"", "rm -rf /"));
        assert!(contains("rm -rf '/'", "rm -rf /"));
        assert!(contains("\"rm\" -rf /", "rm -rf /"));
        assert!(contains("rm -r\"f\" /", "rm -rf /"));
    }

    #[test]
    fn single_quoted_text_is_not_a_command() {
        // A literal backtick inside single quotes is printed, not executed.
        let targets = expand("echo '`rm -rf /`'");
        assert!(
            !targets.iter().any(|t| t == "rm -rf /"),
            "single-quoted text must not become a command: {targets:?}"
        );
    }

    #[test]
    fn escaped_operators_do_not_split() {
        let targets = expand("echo a\\;b");
        assert_eq!(targets.len(), 2, "{targets:?}");
        assert!(targets.contains(&"echo a;b".to_string()), "{targets:?}");
    }

    #[test]
    fn control_operators_split_commands() {
        for command in [
            "ls && rm -rf /",
            "ls || rm -rf /",
            "ls ; rm -rf /",
            "ls | rm -rf /",
            "ls & rm -rf /",
            "ls\nrm -rf /",
        ] {
            assert!(contains(command, "rm -rf /"), "{command}");
        }
    }

    #[test]
    fn wrappers_and_payloads_are_unwrapped() {
        for command in [
            "sudo rm -rf /",
            "env rm -rf /",
            "timeout 5 rm -rf /",
            "nohup rm -rf /",
            "xargs rm -rf /",
            "/usr/bin/sudo rm -rf /",
            "eval 'rm -rf /'",
            "bash -c 'rm -rf /'",
            "sh -lc \"rm -rf /\"",
            "sudo -u root bash -c 'rm -rf /'",
            // 2026-08-04: `-o vi` used to end the flag scan before `-c` was
            // seen, so the payload skipped deny expansion entirely.
            "bash -o vi -c 'rm -rf /'",
            "zsh --norcs -c 'rm -rf /'",
        ] {
            assert!(
                contains(command, "rm -rf /"),
                "{command}: {:?}",
                expand(command)
            );
        }
    }

    #[test]
    fn wrapper_head_scan_stops_at_a_real_command() {
        // `echo` prints its arguments; nothing here is executed as a shell.
        let targets = expand("echo bash -c 'rm -rf /'");
        assert!(
            !targets.iter().any(|t| t == "rm -rf /"),
            "arguments of a printing command must not be parsed as code: {targets:?}"
        );
    }

    #[test]
    fn process_and_parameter_substitution_bodies_are_commands() {
        assert!(contains("diff <(rm -rf /) b", "rm -rf /"));
        assert!(contains("echo ${x:-$(rm -rf /)}", "rm -rf /"));
    }

    #[test]
    fn expansion_is_bounded() {
        let deep = "$(".repeat(64) + "rm -rf /" + &")".repeat(64);
        let targets = expand(&deep);
        assert!(targets.len() <= MAX_COMMANDS);
    }

    #[test]
    fn grouping_is_a_command_boundary() {
        assert!(contains("(rm -rf /)", "rm -rf /"));
        assert!(contains("{ rm -rf /; }", "rm -rf /"));
        assert!(contains("(cd /tmp && rm -rf /)", "rm -rf /"));
        // Escaped and quoted parens are operands, not grouping.
        assert!(contains(
            "find . \\( -name a \\) -print",
            "find . ( -name a ) -print"
        ));
    }

    #[test]
    fn plain_command_expands_to_itself() {
        assert_eq!(expand("git status -s"), vec!["git status -s".to_string()]);
    }
}
