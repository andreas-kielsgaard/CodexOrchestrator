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
                _ => return Err(image_error("Unsupported image type")),
            };
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&image.data_base64)
                .map_err(|_| image_error("Image content is not valid base64"))?;
            if bytes.len() > 10 * 1024 * 1024
                || format!("{:x}", Sha256::digest(&bytes)) != image.id
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
