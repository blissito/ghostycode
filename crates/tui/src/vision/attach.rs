//! Inline image attachments for the main chat model.
//!
//! The composer records images as `[Attached image: … at /path]` lines
//! (clipboard paste, `/attach`, drag-and-drop). Historically those lines
//! only carried a path: the bytes never reached the model, so a model that
//! *can* see images stayed blind and had to bounce through `image_analyze`
//! or OCR. This module turns those references into `image_url` content
//! blocks so vision-capable models get the actual pixels.
//!
//! Gating is by model name rather than provider: the same OpenAI-compatible
//! endpoint serves both blind and sighted models, and the model id is the
//! only signal available at message-build time.

use std::path::Path;

use base64::Engine as _;

use crate::models::{ContentBlock, ImageUrlContent};
use crate::tui::file_mention::media_attachment_references;

/// Maximum number of images attached to a single user message. Beyond this
/// the request payload grows faster than any answer improves.
const MAX_INLINE_IMAGES: usize = 4;

/// Maximum size of a single image, in bytes, before base64 expansion.
/// 5 MB of PNG becomes ~6.7 MB of base64 — already a large upload.
const MAX_INLINE_IMAGE_BYTES: u64 = 5 * 1024 * 1024;

/// Whether `model` accepts inline `image_url` content blocks.
///
/// Conservative allowlist by name pattern: a false negative costs the user
/// a fallback to `image_analyze`, while a false positive costs a hard 400
/// from the provider mid-turn.
#[must_use]
pub fn model_supports_inline_images(model: &str) -> bool {
    let model = model.to_ascii_lowercase();
    // Explicit blind-model denies first — `deepseek-*` chat endpoints reject
    // multimodal content outright.
    if model.starts_with("deepseek") {
        return false;
    }
    model.starts_with("kimi-k3")
        || model.starts_with("kimi-k2.6")
        || model.starts_with("kimi-latest")
        || model.contains("-vl")
        || model.contains("vision")
        || model.starts_with("gpt-4o")
        || model.starts_with("gpt-5")
        || model.starts_with("o3")
        || model.starts_with("o4")
        || model.starts_with("claude-")
        || model.starts_with("gemini-")
        || model.starts_with("glm-4.5v")
        || model.starts_with("glm-4.6v")
        || model.starts_with("glm-5")
}

/// MIME type for an image path, or `None` when the extension is not an
/// image format the OpenAI-compatible wire format accepts.
fn image_mime_type(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "bmp" => Some("image/bmp"),
        _ => None,
    }
}

/// Build `image_url` content blocks for every image attachment referenced
/// in `text`.
///
/// Unreadable, oversized, or non-image attachments are skipped silently —
/// the `[Attached …]` line stays in the text, so the model still sees the
/// path and can fall back to `image_analyze` or `read_file` OCR.
#[must_use]
pub fn inline_image_blocks(text: &str) -> Vec<ContentBlock> {
    let mut blocks = Vec::new();
    for reference in media_attachment_references(text) {
        if blocks.len() >= MAX_INLINE_IMAGES {
            break;
        }
        if reference.kind != "image" {
            continue;
        }
        let path = Path::new(&reference.path);
        let Some(mime) = image_mime_type(path) else {
            continue;
        };
        let too_big = std::fs::metadata(path)
            .map(|meta| meta.len() > MAX_INLINE_IMAGE_BYTES)
            .unwrap_or(true);
        if too_big {
            continue;
        }
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        blocks.push(ContentBlock::ImageUrl {
            image_url: ImageUrlContent {
                url: format!("data:{mime};base64,{encoded}"),
            },
        });
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kimi_k3_sees_images_but_deepseek_does_not() {
        assert!(model_supports_inline_images("kimi-k3"));
        assert!(model_supports_inline_images("GLM-5.2"));
        assert!(!model_supports_inline_images("deepseek-v4-pro"));
    }

    #[test]
    fn builds_data_url_block_for_attached_png() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let image = dir.path().join("shot.png");
        std::fs::write(&image, b"fake png bytes").expect("write fixture");
        let text = format!(
            "mira esto\n[Attached image: 8x4 PNG at {}]",
            image.display()
        );

        let blocks = inline_image_blocks(&text);

        assert_eq!(blocks.len(), 1);
        let ContentBlock::ImageUrl { image_url } = &blocks[0] else {
            panic!("expected an image_url block");
        };
        assert!(image_url.url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn skips_missing_and_non_image_attachments() {
        let text = "[Attached image: /nope/missing.png]\n[Attached video: /tmp/clip.mp4]";
        assert!(inline_image_blocks(text).is_empty());
    }
}
