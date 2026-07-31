use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Descriptor {
    version: u8,
    address: String,
    protected_capability: String,
    process_id: u32,
}

#[cfg(windows)]
#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Foreground {
    window: usize,
    process_id: u32,
}

#[cfg(not(windows))]
#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Foreground {
    window: usize,
    process_id: u32,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut arguments = env::args().skip(1);
    let runtime_root = PathBuf::from(arguments.next().ok_or_else(usage)?);
    let action = arguments.next().ok_or_else(usage)?;
    let descriptor = read_descriptor(&runtime_root)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| "Create the background proof client.".to_string())?;
    if action == "watch" {
        let operation_ref = arguments.next().ok_or_else(usage)?;
        loop {
            let response = send(
                &client,
                &descriptor,
                json!({"kind": "operation", "operationRef": operation_ref}),
            )
            .await?;
            println!(
                "{}",
                serde_json::to_string(&response)
                    .map_err(|_| "Encode the proof response.".to_string())?
            );
            if response["response"]["kind"] == "operation"
                && response["response"]["value"]["progress"]["state"] != "pending"
            {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        return Ok(());
    }
    let command = command(&action, arguments.collect())?;
    let response = send(&client, &descriptor, command).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&response)
            .map_err(|_| "Encode the proof response.".to_string())?
    );
    Ok(())
}

fn command(action: &str, arguments: Vec<String>) -> Result<Value, String> {
    let value = |index: usize| arguments.get(index).cloned().ok_or_else(usage);
    match action {
        "sources" => Ok(json!({"kind": "list_sources"})),
        "instances" => Ok(json!({"kind": "list_instances"})),
        "launcher" => Ok(json!({"kind": "navigate_launcher"})),
        "launcher-detail" => Ok(json!({
            "kind": "navigate_launcher_detail",
            "instanceRef": value(0)?,
        })),
        "prepare" => Ok(json!({
            "kind": "begin_prepare",
            "sourceRef": value(0)?,
            "name": value(1)?,
        })),
        "build" => Ok(json!({"kind": "begin_build", "instanceRef": value(0)?})),
        "open" => Ok(json!({
            "kind": "begin_open",
            "instanceRef": value(0)?,
            "activation": "background_proof",
        })),
        "operation" => Ok(json!({"kind": "operation", "operationRef": value(0)?})),
        "status" | "stop" | "recover" => Ok(json!({"kind": action, "instanceRef": value(0)?})),
        "navigate" => Ok(json!({
            "kind": "navigate",
            "instanceRef": value(0)?,
            "route": value(1)?,
        })),
        "context" => Ok(json!({"kind": "worktree_context", "instanceRef": value(0)?})),
        "files" => Ok(json!({"kind": "file_review", "instanceRef": value(0)?})),
        "detail" => Ok(json!({"kind": "build_detail", "instanceRef": value(0)?})),
        _ => Err(usage()),
    }
}

async fn send(client: &Client, descriptor: &Descriptor, command: Value) -> Result<Value, String> {
    let before = foreground();
    let capability = unprotect_capability(&descriptor.protected_capability)?;
    let response = client
        .post(format!("{}/v1/command", descriptor.address))
        .header("x-codex-review-capability", capability)
        .json(&json!({
            "requestRef": format!("proof-request-{}", Uuid::new_v4().simple()),
            "command": command,
        }))
        .send()
        .await
        .map_err(|_| "The isolated background controller is unavailable.".to_string())?;
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|_| "The background controller returned an invalid response.".to_string())?;
    let after = foreground();
    if before.window != after.window || before.process_id != after.process_id {
        return Err(format!(
            "Foreground ownership changed during the proof operation (before window {} process {}, after window {} process {}).",
            before.window, before.process_id, after.window, after.process_id
        ));
    }
    if !status.is_success() {
        return Err(format!(
            "The semantic background operation was rejected ({status}): {}",
            body["error"].as_str().unwrap_or("safe error")
        ));
    }
    Ok(json!({
        "controllerProcessId": descriptor.process_id,
        "foreground": {
            "before": before,
            "after": after,
            "unchanged": true,
        },
        "response": body,
    }))
}

fn read_descriptor(runtime_root: &Path) -> Result<Descriptor, String> {
    if !runtime_root.is_absolute() {
        return Err("The isolated review runtime root must be absolute.".into());
    }
    let directory = runtime_root.join("debug-controller");
    let mut descriptors = fs::read_dir(directory)
        .map_err(|_| "The isolated background controller is not enabled.".to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("controller-") && name.ends_with(".json"))
        })
        .collect::<Vec<_>>();
    descriptors.sort();
    if descriptors.len() != 1 {
        return Err("Expected exactly one isolated background controller descriptor.".into());
    }
    let path = descriptors.pop().expect("one descriptor");
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| "Inspect the background controller descriptor.".to_string())?;
    if !metadata.file_type().is_file() || metadata.len() > 8_192 {
        return Err("The background controller descriptor is invalid.".into());
    }
    let descriptor: Descriptor = serde_json::from_slice(
        &fs::read(path).map_err(|_| "Read the background controller descriptor.".to_string())?,
    )
    .map_err(|_| "The background controller descriptor is invalid.".to_string())?;
    if descriptor.version != 1
        || descriptor.protected_capability.len() < 64
        || !descriptor
            .protected_capability
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || !descriptor.address.starts_with("http://127.0.0.1:")
    {
        return Err("The background controller descriptor is invalid.".into());
    }
    Ok(descriptor)
}

#[cfg(windows)]
fn unprotect_capability(value: &str) -> Result<String, String> {
    use std::{ptr::null, slice};
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    let mut encoded = hex_decode(value)?;
    let input = CRYPT_INTEGER_BLOB {
        cbData: encoded.len() as u32,
        pbData: encoded.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    if unsafe {
        CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            null(),
            null(),
            null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    } == 0
    {
        return Err("Unlock the current-user background controller capability.".into());
    }
    let bytes = unsafe { slice::from_raw_parts(output.pbData, output.cbData as usize) };
    let capability = String::from_utf8(bytes.to_vec())
        .map_err(|_| "The background controller capability is invalid.".to_string())?;
    unsafe {
        LocalFree(output.pbData as _);
    }
    Ok(capability)
}

#[cfg(not(windows))]
fn unprotect_capability(value: &str) -> Result<String, String> {
    String::from_utf8(hex_decode(value)?)
        .map_err(|_| "The background controller capability is invalid.".to_string())
}

fn hex_decode(value: &str) -> Result<Vec<u8>, String> {
    if value.len() % 2 != 0 {
        return Err("The background controller capability is invalid.".into());
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_value(pair[0])?;
            let low = hex_value(pair[1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn hex_value(value: u8) -> Result<u8, String> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err("The background controller capability is invalid.".into()),
    }
}

#[cfg(windows)]
fn foreground() -> Foreground {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };
    let window = unsafe { GetForegroundWindow() };
    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut process_id);
    }
    Foreground {
        window: window as usize,
        process_id,
    }
}

#[cfg(not(windows))]
fn foreground() -> Foreground {
    Foreground {
        window: 0,
        process_id: 0,
    }
}

fn usage() -> String {
    "Usage: cargo run --example worktree_review_controller -- <isolated-runtime-root> <sources|instances|launcher|launcher-detail|prepare|build|open|operation|watch|status|stop|recover|navigate|context|files|detail> [opaque arguments]".into()
}
