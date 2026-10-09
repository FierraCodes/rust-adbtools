//! OS-specific helpers: data directories and launching external tools.

use std::path::PathBuf;
use std::process::Command;

pub fn cache_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("rusty-adb")
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("rusty-adb")
}

pub fn default_out_dir() -> String {
    dirs::picture_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .display()
        .to_string()
}

/// Find an executable on PATH, plus common install dirs that GUI apps don't inherit
/// (macOS apps launched from Finder get a minimal PATH, so Homebrew would be missed).
fn find_tool(name: &str) -> PathBuf {
    let exe = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    if cfg!(target_os = "macos") {
        dirs.extend(["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin"].map(PathBuf::from));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join("Library/Android/sdk/platform-tools"));
        dirs.push(home.join("Android/Sdk/platform-tools"));
        dirs.push(home.join("AppData/Local/Android/Sdk/platform-tools"));
    }
    dirs.into_iter()
        .map(|d| d.join(&exe))
        .find(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from(name))
}

/// A `Command` for `adb` / `scrcpy` that doesn't flash a console window on Windows.
pub fn command(name: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(find_tool(name));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}
