use serde::Serialize;

const TARGET: &str = "dev.codex-orchestrator.app/jev/api-key";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    saved: bool,
    environment_available: bool,
}

fn environment_key() -> Option<String> {
    std::env::var("TYPESAFE_API_KEY")
        .ok()
        .map(|key| key.trim().to_owned())
        .filter(|key| !key.is_empty())
}

pub fn status() -> Result<CredentialStatus, String> {
    Ok(CredentialStatus {
        saved: platform::read(TARGET)?.is_some(),
        environment_available: environment_key().is_some(),
    })
}

pub fn resolve() -> Result<String, String> {
    platform::read(TARGET)?
        .or_else(environment_key)
        .ok_or_else(|| "Set the JEV API key in Technical Settings â†’ JEV".into())
}

pub fn save(key: &str) -> Result<CredentialStatus, String> {
    platform::write(TARGET, key.trim())?;
    status()
}

pub fn remove() -> Result<CredentialStatus, String> {
    platform::delete(TARGET)?;
    status()
}

#[cfg(test)]
mod tests {
    #[test]
    fn status_exposes_only_presence_flags() {
        let value = serde_json::to_value(super::CredentialStatus {
            saved: true,
            environment_available: false,
        })
        .unwrap();
        assert_eq!(
            value,
            serde_json::json!({"saved": true, "environmentAvailable": false})
        );
    }
}

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::{
        Foundation::{GetLastError, ERROR_NOT_FOUND},
        Security::Credentials::{
            CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW,
            CRED_MAX_CREDENTIAL_BLOB_SIZE, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
        },
    };

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn error(operation: &str) -> String {
        format!(
            "Windows Credential Manager could not {operation} the JEV key (error {})",
            unsafe { GetLastError() }
        )
    }

    pub(super) fn read(target: &str) -> Result<Option<String>, String> {
        let target = wide(target);
        let mut credential = std::ptr::null_mut();
        // CredReadW owns the returned allocation until CredFree; copy only its blob.
        unsafe {
            if CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) == 0 {
                return if GetLastError() == ERROR_NOT_FOUND {
                    Ok(None)
                } else {
                    Err(error("read"))
                };
            }
            let value = if (*credential).CredentialBlobSize == 0 {
                Err("The saved JEV key is empty; replace it in Technical Settings â†’ JEV".into())
            } else {
                let blob = std::slice::from_raw_parts(
                    (*credential).CredentialBlob,
                    (*credential).CredentialBlobSize as usize,
                );
                String::from_utf8(blob.to_vec()).map(Some).map_err(|_| {
                    "The saved JEV key is invalid; replace it in Technical Settings â†’ JEV".into()
                })
            };
            CredFree(credential.cast());
            value
        }
    }

    pub(super) fn write(target: &str, key: &str) -> Result<(), String> {
        if key.is_empty()
            || key.chars().any(char::is_whitespace)
            || key.len() > CRED_MAX_CREDENTIAL_BLOB_SIZE as usize
        {
            return Err(
                "Enter a nonempty JEV API key without whitespace (maximum 2560 bytes)".into(),
            );
        }
        let mut target = wide(target);
        let mut user = wide("Orchid JEV");
        let mut blob = key.as_bytes().to_vec();
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            UserName: user.as_mut_ptr(),
            ..Default::default()
        };
        let result = if unsafe { CredWriteW(&credential, 0) } == 0 {
            Err(error("save"))
        } else {
            Ok(())
        };
        blob.fill(0);
        result
    }

    pub(super) fn delete(target: &str) -> Result<(), String> {
        let target = wide(target);
        if unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) } == 0
            && unsafe { GetLastError() } != ERROR_NOT_FOUND
        {
            return Err(error("remove"));
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn credential_manager_persists_replaces_and_removes_key() {
            let target = format!("dev.codex-orchestrator.app/test/{}", uuid::Uuid::new_v4());
            assert_eq!(read(&target).unwrap(), None);
            write(&target, "test-key-one").unwrap();
            assert_eq!(read(&target).unwrap().as_deref(), Some("test-key-one"));
            write(&target, "test-key-two").unwrap();
            assert_eq!(read(&target).unwrap().as_deref(), Some("test-key-two"));
            assert!(write(&target, " ").is_err());
            assert_eq!(read(&target).unwrap().as_deref(), Some("test-key-two"));
            delete(&target).unwrap();
            assert_eq!(read(&target).unwrap(), None);
            delete(&target).unwrap();
        }
    }
}

#[cfg(not(windows))]
mod platform {
    pub(super) fn read(_: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
    pub(super) fn write(_: &str, _: &str) -> Result<(), String> {
        Err("Saving JEV credentials requires Windows Credential Manager".into())
    }
    pub(super) fn delete(_: &str) -> Result<(), String> {
        Err("Removing JEV credentials requires Windows Credential Manager".into())
    }
}
