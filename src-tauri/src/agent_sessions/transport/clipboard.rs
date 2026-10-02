//! Clipboard reads occur only on an explicit composer paste/menu interaction.
#[derive(serde::Serialize)]
pub(crate) struct ComposerClipboard {
    text: Option<String>,
    paths: Vec<String>,
}

#[tauri::command]
pub(crate) async fn read_agent_session_clipboard() -> Result<ComposerClipboard, String> {
    tauri::async_runtime::spawn_blocking(read)
        .await
        .map_err(|e| e.to_string())?
}
#[cfg(not(windows))]
fn read() -> Result<ComposerClipboard, String> {
    Err("Native file clipboard is unavailable on this platform".into())
}

#[cfg(windows)]
fn read() -> Result<ComposerClipboard, String> {
    use windows_sys::Win32::{
        System::{
            DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard},
            Memory::{GlobalLock, GlobalSize, GlobalUnlock},
        },
        UI::Shell::DragQueryFileW,
    };
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return Err("Clipboard is busy; try again".into());
        }
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                unsafe {
                    CloseClipboard();
                }
            }
        }
        let _guard = Guard;
        let mut paths = Vec::new();
        let files = GetClipboardData(15); // CF_HDROP
        if !files.is_null() {
            let count = DragQueryFileW(files, u32::MAX, std::ptr::null_mut(), 0);
            for index in 0..count.min(4) {
                let length = DragQueryFileW(files, index, std::ptr::null_mut(), 0);
                if length > 32768 {
                    continue;
                }
                let mut buffer = vec![0u16; length as usize + 1];
                DragQueryFileW(files, index, buffer.as_mut_ptr(), buffer.len() as u32);
                paths.push(String::from_utf16_lossy(&buffer[..length as usize]));
            }
        }
        let handle = GetClipboardData(13); // CF_UNICODETEXT
        let text = if handle.is_null() {
            None
        } else {
            let size = GlobalSize(handle);
            if size > 20 * 1024 * 1024 {
                return Err("Clipboard text exceeds 10 MB of characters".into());
            }
            let ptr = GlobalLock(handle) as *const u16;
            if ptr.is_null() {
                None
            } else {
                let words = std::slice::from_raw_parts(ptr, size / 2);
                let end = words.iter().position(|v| *v == 0).unwrap_or(words.len());
                let text = String::from_utf16_lossy(&words[..end]);
                GlobalUnlock(handle);
                Some(text)
            }
        };
        Ok(ComposerClipboard { text, paths })
    }
}
