//! Foreground application detection for per-app dictation profiles.

/// Returns the executable name (e.g. "WhatsApp.exe") of the app that owns
/// the foreground window, lowercased. `None` when it cannot be determined —
/// callers must treat that as "no profile matches".
#[cfg(target_os = "windows")]
pub fn foreground_app_exe() -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        result.ok()?;

        let full_path = String::from_utf16_lossy(&buf[..len as usize]);
        full_path
            .rsplit(['\\', '/'])
            .next()
            .map(|name| name.to_lowercase())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn foreground_app_exe() -> Option<String> {
    None
}
