pub mod app_server;
pub mod capabilities;
pub mod host;
pub mod options;
pub mod protocol;
pub mod runtime_profile;
pub use capabilities::resolve_program;

use base64::Engine;
use sha2::{Digest, Sha256};

pub fn materialize_images(
    images: &[crate::contracts::RuntimeImageAttachment],
) -> Result<Vec<std::path::PathBuf>, crate::contracts::RuntimePortError> {
    materialize_attachments(
        &images
            .iter()
            .filter(|a| attachment_is_image(a))
            .cloned()
            .collect::<Vec<_>>(),
    )
}

pub fn attachment_is_image(attachment: &crate::contracts::RuntimeImageAttachment) -> bool {
    matches!(
        attachment.media_type.as_str(),
        "image/png" | "image/jpeg" | "image/gif" | "image/webp"
    )
}

pub fn file_attachment_context(
    attachments: &[crate::contracts::RuntimeImageAttachment],
) -> Result<String, crate::contracts::RuntimePortError> {
    let files = attachments
        .iter()
        .filter(|a| !attachment_is_image(a))
        .cloned()
        .collect::<Vec<_>>();
    let paths = materialize_attachments(&files)?;
    Ok(files
        .iter()
        .zip(paths)
        .map(|(file, path)| {
            format!(
                "Attached file {} is available on this execution device at {}.",
                serde_json::to_string(&file.name).unwrap(),
                serde_json::to_string(&path.to_string_lossy()).unwrap()
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

fn materialize_attachments(
    images: &[crate::contracts::RuntimeImageAttachment],
) -> Result<Vec<std::path::PathBuf>, crate::contracts::RuntimePortError> {
    let root = std::env::temp_dir().join("orchid-agent-images");
    std::fs::create_dir_all(&root).map_err(|error| image_error(error.to_string()))?;
    images
        .iter()
        .map(|image| {
            let extension = match image.media_type.as_str() {
                "image/png" => "png",
                "image/jpeg" => "jpg",
                "image/gif" => "gif",
                "image/webp" => "webp",
                _ => std::path::Path::new(&image.name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .filter(|e| e.len() <= 16 && e.bytes().all(|b| b.is_ascii_alphanumeric()))
                    .unwrap_or("bin"),
            };
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&image.data_base64)
                .map_err(|_| image_error("Image content is not valid base64"))?;
            if bytes.len() > 10 * 1024 * 1024 || format!("{:x}", Sha256::digest(&bytes)) != image.id
            {
                return Err(image_error("Image content failed integrity validation"));
            }
            let path = root.join(format!("{}.{}", image.id, extension));
            if !path.exists() {
                std::fs::write(&path, bytes).map_err(|error| image_error(error.to_string()))?;
            }
            Ok(path)
        })
        .collect()
}

fn image_error(message: impl Into<String>) -> crate::contracts::RuntimePortError {
    crate::contracts::RuntimePortError::new(
        crate::contracts::RuntimePortErrorKind::Unavailable,
        message,
    )
}

#[cfg(test)]
mod image_tests {
    use super::*;
    #[test]
    fn ordinary_files_are_materialized_as_verified_files_not_native_images() {
        let bytes = b"portable text file";
        let attachment = crate::contracts::RuntimeImageAttachment {
            id: format!("{:x}", Sha256::digest(bytes)),
            name: "notes.txt".into(),
            media_type: "text/plain".into(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        };
        assert!(materialize_images(&[attachment.clone()])
            .unwrap()
            .is_empty());
        let context = file_attachment_context(&[attachment.clone()]).unwrap();
        assert!(context.contains("notes.txt"));
        let path = std::env::temp_dir()
            .join("orchid-agent-images")
            .join(format!("{}.txt", attachment.id));
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }

    #[test]
    fn portable_image_is_verified_and_materialized_on_this_device() {
        let bytes = b"orchid-image";
        let id = format!("{:x}", Sha256::digest(bytes));
        let paths = materialize_images(&[crate::contracts::RuntimeImageAttachment {
            id: id.clone(),
            name: "reference.png".into(),
            media_type: "image/png".into(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        }])
        .unwrap();
        assert_eq!(std::fs::read(&paths[0]).unwrap(), bytes);
        assert!(paths[0].ends_with(format!("{id}.png")));
        let _ = std::fs::remove_file(&paths[0]);
    }

    #[test]
    fn portable_image_rejects_a_mismatched_identity() {
        let error = materialize_images(&[crate::contracts::RuntimeImageAttachment {
            id: "0".repeat(64),
            name: "reference.png".into(),
            media_type: "image/png".into(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(b"different"),
        }])
        .unwrap_err();
        assert!(error.message.contains("integrity"));
    }
}

#[cfg(test)]
mod protocol_tests;
