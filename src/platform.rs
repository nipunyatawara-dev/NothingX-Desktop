use std::{fs, io, path::PathBuf};

#[cfg(windows)]
pub const WM_REOPEN: u32 = 0x8001; // WM_APP + 1

pub fn data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let base = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(std::env::temp_dir);
        base.join("Library/Application Support/NothingX Desktop")
    }
    #[cfg(windows)]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .map(PathBuf::from)
                    .filter(|p| p.is_absolute())
                    .map(|p| p.join("AppData/Local"))
            })
            .unwrap_or_else(std::env::temp_dir);
        base.join("NothingX Desktop")
    }
}

pub fn single_instance(simulation: bool) -> io::Result<fs::File> {
    let dir = data_dir();
    fs::create_dir_all(&dir)?;
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join(if simulation {
            "simulation.lock"
        } else {
            "app.lock"
        }))?;
    fs2::FileExt::try_lock_exclusive(&file)?;
    Ok(file)
}

pub fn notify_running_instance() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            ASFW_ANY, AllowSetForegroundWindow, FindWindowW, PostMessageW,
        };
        let class: Vec<u16> = "NothingXDesktopWindow\0".encode_utf16().collect();
        let title: Vec<u16> = "NothingX Desktop\0".encode_utf16().collect();
        unsafe {
            let hwnd = FindWindowW(class.as_ptr(), title.as_ptr());
            if !hwnd.is_null() {
                AllowSetForegroundWindow(ASFW_ANY);
                PostMessageW(hwnd, WM_REOPEN, 0, 0);
            }
        }
    }
}
