//! Background GitHub Releases update check and verified in-place upgrade.
//! Uses system `curl` and OS checksum tools to keep the binary small and dependency-free.
use serde_json::Value as Json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/nipunyatawara-dev/NothingX-Desktop/releases/latest";
const ALLOWED_REPO_PREFIX: &str = "https://github.com/nipunyatawara-dev/NothingX-Desktop/";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub version: String,
    pub tag: String,
    pub asset_name: String,
    pub asset_url: String,
    pub checksum_url: String,
    pub release_url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking {
        manual: bool,
    },
    UpToDate,
    Available(ReleaseInfo),
    Downloading {
        version: String,
    },
    Verifying {
        version: String,
    },
    Installing {
        version: String,
    },
    OpenedInstaller {
        version: String,
    },
    Error {
        message: String,
        manual: bool,
    },
}

impl UpdateStatus {
    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            Self::Checking { .. }
                | Self::Downloading { .. }
                | Self::Verifying { .. }
                | Self::Installing { .. }
        )
    }
    pub fn available_release(&self) -> Option<&ReleaseInfo> {
        match self {
            Self::Available(info) => Some(info),
            _ => None,
        }
    }
}

pub enum InstallOutcome {
    RestartNow,
    OpenedManually,
}

pub fn expected_asset_suffix() -> Option<&'static str> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        Some("-macos-arm64.dmg")
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        Some("-macos-intel.dmg")
    }
    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        Some("-windows-x64-setup.exe")
    }
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(windows, target_arch = "x86_64")
    )))]
    {
        None
    }
}

pub fn parse_version(raw: &str) -> Option<(u32, u32, u32)> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    if trimmed.is_empty() || trimmed.contains('-') || trimmed.contains('+') {
        return None;
    }
    let mut parts = trimmed.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = match parts.next() {
        Some(s) => s.parse::<u32>().ok()?,
        None => 0,
    };
    let patch = match parts.next() {
        Some(s) => s.parse::<u32>().ok()?,
        None => 0,
    };
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

pub fn is_newer(current: &str, candidate: &str) -> bool {
    match (parse_version(current), parse_version(candidate)) {
        (Some(cur), Some(cand)) => cand > cur,
        _ => false,
    }
}

pub fn is_allowed_download_url(url: &str) -> bool {
    url.starts_with(ALLOWED_REPO_PREFIX) && !url.bytes().any(|b| b.is_ascii_control() || b == b' ')
}

pub fn parse_latest_release(
    body: &str,
    current_version: &str,
    asset_suffix: &str,
) -> Result<Option<ReleaseInfo>, String> {
    let json: Json = serde_json::from_str(body).map_err(|_| "Invalid release metadata")?;
    if json["draft"].as_bool().unwrap_or(false) || json["prerelease"].as_bool().unwrap_or(false) {
        return Ok(None);
    }
    let tag = json["tag_name"]
        .as_str()
        .ok_or("Missing release tag")?
        .trim();
    if !is_newer(current_version, tag) {
        return Ok(None);
    }
    let Some(assets) = json["assets"].as_array() else {
        return Ok(None);
    };
    let asset = assets.iter().find(|a| {
        let Some(name) = a["name"].as_str() else {
            return false;
        };
        let Some(url) = a["browser_download_url"].as_str() else {
            return false;
        };
        name.starts_with("NothingX-Desktop-")
            && name.ends_with(asset_suffix)
            && is_allowed_download_url(url)
    });
    let Some(asset) = asset else {
        // Release exists (e.g. macOS-only) without a matching asset for this platform.
        return Ok(None);
    };
    let preferred_checksums: &[&str] = if asset_suffix.ends_with(".dmg") {
        &["SHA256SUMS-macos.txt", "SHA256SUMS.txt"]
    } else {
        &["SHA256SUMS.txt"]
    };
    let checksum_url = preferred_checksums
        .iter()
        .find_map(|wanted| {
            assets.iter().find_map(|a| {
                let name = a["name"].as_str()?;
                let url = a["browser_download_url"].as_str()?;
                (name == *wanted && is_allowed_download_url(url)).then(|| url.to_string())
            })
        })
        .ok_or("Release checksum file is missing")?;
    let release_url = json["html_url"]
        .as_str()
        .filter(|u| is_allowed_download_url(u))
        .unwrap_or("https://github.com/nipunyatawara-dev/NothingX-Desktop/releases/latest")
        .to_string();
    Ok(Some(ReleaseInfo {
        version: tag.trim_start_matches(['v', 'V']).to_string(),
        tag: tag.to_string(),
        asset_name: asset["name"].as_str().unwrap_or_default().to_string(),
        asset_url: asset["browser_download_url"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        checksum_url,
        release_url,
    }))
}

pub fn extract_sha256(checksums: &str, asset_name: &str) -> Option<String> {
    for line in checksums.lines() {
        let mut parts = line.split_whitespace();
        let Some(hash) = parts.next() else {
            continue;
        };
        let Some(file) = parts.next() else {
            continue;
        };
        let cleaned = file.trim_start_matches('*');
        let basename = cleaned.rsplit(['/', '\\']).next().unwrap_or(cleaned);
        if basename == asset_name && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Some(hash.to_ascii_lowercase());
        }
    }
    None
}

pub fn check_latest() -> Result<Option<ReleaseInfo>, String> {
    let Some(suffix) = expected_asset_suffix() else {
        return Ok(None);
    };
    let body = fetch_text(LATEST_RELEASE_API)?;
    parse_latest_release(&body, env!("CARGO_PKG_VERSION"), suffix)
}

fn curl_bin() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "/usr/bin/curl"
    }
    #[cfg(windows)]
    {
        "curl.exe"
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        "curl"
    }
}

fn user_agent() -> String {
    format!("NothingX-Desktop/{}", env!("CARGO_PKG_VERSION"))
}

fn fetch_text(url: &str) -> Result<String, String> {
    let output = Command::new(curl_bin())
        .args([
            "-fsSL",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--connect-timeout",
            "10",
            "--max-time",
            "20",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            &format!("User-Agent: {}", user_agent()),
            url,
        ])
        .output()
        .map_err(|_| "Could not run update check".to_string())?;
    if !output.status.success() {
        return Err("Update check failed".into());
    }
    String::from_utf8(output.stdout).map_err(|_| "Invalid response encoding".into())
}

fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    if !is_allowed_download_url(url) {
        return Err("Blocked untrusted download URL".into());
    }
    let status = Command::new(curl_bin())
        .args([
            "-fsSL",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--connect-timeout",
            "15",
            "--max-time",
            "300",
            "-H",
            &format!("User-Agent: {}", user_agent()),
            "-o",
        ])
        .arg(dest)
        .arg(url)
        .status()
        .map_err(|_| "Could not start download".to_string())?;
    if !status.success() {
        return Err("Update download failed".into());
    }
    Ok(())
}

fn file_sha256(path: &Path) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("/usr/bin/shasum")
            .args(["-a", "256"])
            .arg(path)
            .output()
            .map_err(|_| "Could not verify checksum".to_string())?;
        if !output.status.success() {
            return Err("Checksum calculation failed".into());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let hash = text
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(hash);
        }
        Err("Unexpected checksum output".into())
    }
    #[cfg(windows)]
    {
        let output = Command::new("certutil")
            .arg("-hashfile")
            .arg(path)
            .arg("SHA256")
            .output()
            .map_err(|_| "Could not verify checksum".to_string())?;
        if !output.status.success() {
            return Err("Checksum calculation failed".into());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let trimmed: String = line.chars().filter(|c| !c.is_whitespace()).collect();
            if trimmed.len() == 64 && trimmed.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Ok(trimmed.to_ascii_lowercase());
            }
        }
        Err("Unexpected checksum output".into())
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = path;
        Err("Checksum verification unsupported on this OS".into())
    }
}

fn temp_update_dir(base_data_dir: &Path) -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let dir = base_data_dir
        .join("updates")
        .join(format!("run-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir).map_err(|_| "Could not create update folder".to_string())?;
    Ok(dir)
}

pub fn download_and_install(
    release: &ReleaseInfo,
    data_dir: &Path,
    mut on_status: impl FnMut(UpdateStatus),
) -> Result<InstallOutcome, String> {
    if !is_allowed_download_url(&release.asset_url)
        || !is_allowed_download_url(&release.checksum_url)
    {
        return Err("Blocked untrusted release URL".into());
    }
    let work = temp_update_dir(data_dir)?;
    let asset_path = work.join(&release.asset_name);

    on_status(UpdateStatus::Downloading {
        version: release.version.clone(),
    });
    if let Err(e) = download_file(&release.asset_url, &asset_path) {
        let _ = fs::remove_dir_all(&work);
        return Err(e);
    }

    on_status(UpdateStatus::Verifying {
        version: release.version.clone(),
    });
    let checksums = match fetch_text(&release.checksum_url) {
        Ok(text) => text,
        Err(e) => {
            let _ = fs::remove_dir_all(&work);
            return Err(e);
        }
    };
    let Some(expected) = extract_sha256(&checksums, &release.asset_name) else {
        let _ = fs::remove_dir_all(&work);
        return Err("Checksum missing for asset".into());
    };
    let actual = match file_sha256(&asset_path) {
        Ok(hash) => hash,
        Err(e) => {
            let _ = fs::remove_dir_all(&work);
            return Err(e);
        }
    };
    if actual != expected {
        let _ = fs::remove_dir_all(&work);
        return Err("Update checksum mismatch".into());
    }

    on_status(UpdateStatus::Installing {
        version: release.version.clone(),
    });

    #[cfg(target_os = "macos")]
    {
        install_macos(&work, &asset_path)
    }
    #[cfg(windows)]
    {
        install_windows(&asset_path)
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = fs::remove_dir_all(&work);
        Err("Auto-install is unsupported on this OS".into())
    }
}

#[cfg(target_os = "macos")]
fn install_macos(work: &Path, dmg_path: &Path) -> Result<InstallOutcome, String> {
    struct MountGuard(PathBuf);
    impl Drop for MountGuard {
        fn drop(&mut self) {
            let _ = Command::new("/usr/bin/hdiutil")
                .args(["detach", "-quiet"])
                .arg(&self.0)
                .status();
        }
    }

    let current_bundle = std::env::current_exe().ok().and_then(|exe| {
        exe.ancestors()
            .find(|p| p.extension().and_then(|e| e.to_str()) == Some("app"))
            .map(Path::to_path_buf)
    });
    let can_replace = current_bundle.as_ref().is_some_and(|bundle| {
        !bundle.starts_with("/Volumes/") && bundle.parent().is_some_and(dir_is_writable)
    });
    if !can_replace {
        let _ = Command::new("/usr/bin/open").arg(dmg_path).spawn();
        return Ok(InstallOutcome::OpenedManually);
    }
    let target_bundle = current_bundle.unwrap();

    let mount = work.join("mount");
    fs::create_dir_all(&mount).map_err(|_| "Could not create mount folder".to_string())?;
    let attach = Command::new("/usr/bin/hdiutil")
        .args(["attach", "-readonly", "-nobrowse", "-quiet", "-mountpoint"])
        .arg(&mount)
        .arg(dmg_path)
        .status()
        .map_err(|_| "Could not mount update DMG".to_string())?;
    if !attach.success() {
        let _ = fs::remove_dir_all(work);
        return Err("Could not mount update DMG".into());
    }
    let _guard = MountGuard(mount.clone());

    let mounted_app = mount.join("NothingX Desktop.app");
    if !mounted_app.is_dir() {
        drop(_guard);
        let _ = fs::remove_dir_all(work);
        return Err("Update DMG missing app bundle".into());
    }
    let verify = Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(&mounted_app)
        .status()
        .map_err(|_| "Could not verify app signature".to_string())?;
    if !verify.success() {
        drop(_guard);
        let _ = fs::remove_dir_all(work);
        return Err("Update signature check failed".into());
    }

    let staged_app = work.join("NothingX Desktop.app");
    let copy = Command::new("/usr/bin/ditto")
        .arg(&mounted_app)
        .arg(&staged_app)
        .status()
        .map_err(|_| "Could not stage update".to_string())?;
    drop(_guard);
    if !copy.success() {
        let _ = fs::remove_dir_all(work);
        return Err("Could not copy updated app".into());
    }
    let _ = fs::remove_file(dmg_path);

    let script_path = work.join("apply-update.sh");
    let script = r#"#!/bin/sh
set -eu
OLD_PID="$1"
TARGET="$2"
STAGED="$3"
WORK_DIR="$4"
BACKUP="${TARGET}.previous"
i=0
while kill -0 "$OLD_PID" 2>/dev/null && [ "$i" -lt 100 ]; do
  sleep 0.1
  i=$((i + 1))
done
rm -rf "$BACKUP"
mv "$TARGET" "$BACKUP"
if mv "$STAGED" "$TARGET"; then
  rm -rf "$BACKUP"
else
  mv "$BACKUP" "$TARGET" || true
  exit 1
fi
/usr/bin/open "$TARGET"
rm -rf "$WORK_DIR"
"#;
    fs::write(&script_path, script).map_err(|_| "Could not write update script".to_string())?;
    Command::new("/bin/sh")
        .arg(&script_path)
        .arg(std::process::id().to_string())
        .arg(&target_bundle)
        .arg(&staged_app)
        .arg(work)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Could not launch update installer".to_string())?;

    Ok(InstallOutcome::RestartNow)
}

#[cfg(target_os = "macos")]
fn dir_is_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".nothingx-write-probe-{}", std::process::id()));
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = fs::remove_file(probe);
            true
        }
        Err(_) => false,
    }
}

#[cfg(windows)]
fn install_windows(setup_path: &Path) -> Result<InstallOutcome, String> {
    Command::new(setup_path)
        .arg("/S")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Could not launch Windows installer".to_string())?;
    Ok(InstallOutcome::RestartNow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parsing_and_comparison_handle_short_and_full_tags() {
        assert_eq!(parse_version("v1.1"), Some((1, 1, 0)));
        assert_eq!(parse_version("1.1.0"), Some((1, 1, 0)));
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("v1.2.0-beta.1"), None);
        assert_eq!(parse_version(""), None);

        assert!(!is_newer("1.1.0", "v1.1"));
        assert!(!is_newer("1.1.0", "v1.0.9"));
        assert!(is_newer("1.1.0", "v1.2"));
        assert!(is_newer("1.1.0", "v1.1.1"));
        assert!(is_newer("1.1.0", "v2.0"));
    }

    #[test]
    fn release_parser_selects_matching_asset_and_skips_missing_platform() {
        let json = r#"{
            "tag_name": "v1.2",
            "html_url": "https://github.com/nipunyatawara-dev/NothingX-Desktop/releases/tag/v1.2",
            "draft": false,
            "prerelease": false,
            "assets": [
                {
                    "name": "NothingX-Desktop-1.2.0-macos-arm64.dmg",
                    "browser_download_url": "https://github.com/nipunyatawara-dev/NothingX-Desktop/releases/download/v1.2/NothingX-Desktop-1.2.0-macos-arm64.dmg"
                },
                {
                    "name": "SHA256SUMS-macos.txt",
                    "browser_download_url": "https://github.com/nipunyatawara-dev/NothingX-Desktop/releases/download/v1.2/SHA256SUMS-macos.txt"
                }
            ]
        }"#;

        let arm = parse_latest_release(json, "1.1.0", "-macos-arm64.dmg")
            .unwrap()
            .expect("arm64 update");
        assert_eq!(arm.version, "1.2");
        assert_eq!(arm.asset_name, "NothingX-Desktop-1.2.0-macos-arm64.dmg");
        assert!(arm.checksum_url.ends_with("SHA256SUMS-macos.txt"));

        // Windows asset missing in a macOS-only release -> no false update prompt.
        assert!(
            parse_latest_release(json, "1.1.0", "-windows-x64-setup.exe")
                .unwrap()
                .is_none()
        );
        // Same version -> up to date.
        assert!(
            parse_latest_release(json, "1.2.0", "-macos-arm64.dmg")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn release_parser_rejects_untrusted_urls_and_extracts_sha256() {
        assert!(is_allowed_download_url(
            "https://github.com/nipunyatawara-dev/NothingX-Desktop/releases/download/v1.2/NothingX-Desktop-1.2.0-macos-arm64.dmg"
        ));
        assert!(!is_allowed_download_url(
            "https://evil.example.com/NothingX-Desktop-1.2.0-macos-arm64.dmg"
        ));
        assert!(!is_allowed_download_url(
            "http://github.com/nipunyatawara-dev/NothingX-Desktop/releases/download/v1.2/a.dmg"
        ));

        let sums = "c178f3576ca968a723f3db541c3fb137d4d18a2517df1dee783827f139642e5f  NothingX-Desktop-1.1.0-macos-arm64.dmg\n\
                    a292a8574421609a44547f3ec872fed8e3714fdc0d351339682029d276d134eb  dist/NothingX-Desktop-1.1.0-macos-intel.dmg\n";
        assert_eq!(
            extract_sha256(sums, "NothingX-Desktop-1.1.0-macos-arm64.dmg").as_deref(),
            Some("c178f3576ca968a723f3db541c3fb137d4d18a2517df1dee783827f139642e5f")
        );
        assert_eq!(
            extract_sha256(sums, "NothingX-Desktop-1.1.0-macos-intel.dmg").as_deref(),
            Some("a292a8574421609a44547f3ec872fed8e3714fdc0d351339682029d276d134eb")
        );
        assert!(extract_sha256(sums, "missing.dmg").is_none());
    }
}
