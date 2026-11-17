use std::{fs, io, path::PathBuf};

pub fn data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
            .join("Library/Application Support/NothingX Desktop")
    }
    #[cfg(windows)]
    {
        PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join("NothingX Desktop")
    }
}

pub fn single_instance(simulation: bool) -> io::Result<fs::File> {
    fs::create_dir_all(data_dir())?;
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(data_dir().join(if simulation {
            "simulation.lock"
        } else {
            "app.lock"
        }))?;
    fs2::FileExt::try_lock_exclusive(&file)?;
    Ok(file)
}
