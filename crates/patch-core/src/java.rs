use crate::error::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const MIN_JAVA_MAJOR: u32 = 17;

/// User-Agent required by PaperMC Fill v3 (also used for consistency elsewhere).
pub const FILL_USER_AGENT: &str =
    "PumpkinPatch/0.1.0 (+https://github.com/Pumpkin-MC/Pumpkin)";

/// Resolve a Java executable for spawning Velocity.
///
/// Uses `configured` when it points at a working `java` binary; otherwise scans common
/// install locations and `PATH`.
pub fn resolve_java_executable(configured: &str) -> Result<PathBuf> {
    let trimmed = configured.trim();
    if !trimmed.is_empty() && trimmed != "java" {
        let path = PathBuf::from(trimmed);
        if java_works(&path) {
            return Ok(path);
        }
    }

    if let Some(path) = find_java_on_path() {
        return Ok(path);
    }

    for candidate in detect_java_candidates() {
        if java_works(&candidate) {
            return Ok(candidate);
        }
    }

    Err(Error::Other(
        "Java 17+ not found. Install a JDK (Eclipse Temurin, Microsoft Build of OpenJDK, etc.) \
         or set the Java path in Settings."
            .into(),
    ))
}

/// Enumerate likely `java.exe` / `java` paths (newest-looking installs first).
pub fn detect_java_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();

    if let Ok(home) = std::env::var("JAVA_HOME") {
        push_java_bin(&mut out, Path::new(&home));
    }

    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let roots = [
            (HKEY_LOCAL_MACHINE, r"SOFTWARE\JavaSoft\JDK"),
            (HKEY_LOCAL_MACHINE, r"SOFTWARE\JavaSoft\JRE"),
            (HKEY_LOCAL_MACHINE, r"SOFTWARE\Eclipse Adoptium\JDK"),
            (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\JDK"),
            (HKEY_CURRENT_USER, r"SOFTWARE\JavaSoft\JDK"),
        ];
        for (hive, path) in roots {
            if let Ok(key) = RegKey::predef(hive).open_subkey(path) {
                for name in key.enum_keys().filter_map(|k| k.ok()) {
                    if let Ok(sub) = key.open_subkey(&name) {
                        if let Ok(home) = sub.get_value::<String, _>("JavaHome") {
                            push_java_bin(&mut out, Path::new(&home));
                        }
                    }
                }
            }
        }

        let program_files = std::env::var_os("ProgramFiles");
        let program_files_x86 = std::env::var_os("ProgramFiles(x86)");
        let local_app = std::env::var_os("LOCALAPPDATA");

        for base in program_files.into_iter().chain(program_files_x86) {
            scan_versioned_dirs(&mut out, Path::new(&base).join("Java"));
            scan_versioned_dirs(&mut out, Path::new(&base).join("Eclipse Adoptium"));
            scan_versioned_dirs(&mut out, Path::new(&base).join("Microsoft"));
            scan_versioned_dirs(&mut out, Path::new(&base).join("Amazon Corretto"));
            scan_versioned_dirs(&mut out, Path::new(&base).join("Zulu"));
        }
        if let Some(local) = local_app {
            scan_versioned_dirs(
                &mut out,
                Path::new(&local).join("Programs").join("Eclipse Adoptium"),
            );
        }

        push_java_bin(
            &mut out,
            Path::new(r"C:\Program Files\Common Files\Oracle\Java\javapath"),
        );
        for entry in windows_path_entries() {
            let candidate = entry.join("java.exe");
            if candidate.is_file() {
                out.push(candidate);
            }
        }
    }

    #[cfg(not(windows))]
    {
        for base in ["/usr/lib/jvm", "/opt/homebrew/opt", "/Library/Java/JavaVirtualMachines"] {
            scan_versioned_dirs(&mut out, Path::new(base));
        }
    }

    out.sort();
    out.dedup();
    out.reverse();
    out
}

fn push_java_bin(out: &mut Vec<PathBuf>, home: &Path) {
    let direct = if cfg!(windows) {
        home.join("java.exe")
    } else {
        home.join("java")
    };
    if direct.is_file() {
        out.push(direct);
        return;
    }
    let exe = if cfg!(windows) {
        home.join("bin").join("java.exe")
    } else {
        home.join("bin").join("java")
    };
    if exe.is_file() {
        out.push(exe);
    }
}

fn scan_versioned_dirs(out: &mut Vec<PathBuf>, parent: PathBuf) {
    if !parent.is_dir() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(&parent) else {
        return;
    };
    let mut dirs: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    dirs.sort();
    for dir in dirs {
        if dir.is_dir() {
            push_java_bin(out, &dir);
            if cfg!(target_os = "macos") {
                push_java_bin(out, &dir.join("Contents").join("Home"));
            }
        }
    }
}

fn find_java_on_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        for program in ["where.exe", "where"] {
            if let Some(path) = find_java_via_where(program) {
                return Some(path);
            }
        }
        for entry in windows_path_entries() {
            let candidate = entry.join("java.exe");
            if java_works(&candidate) {
                return Some(candidate);
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        let output = Command::new("which").arg("java").output().ok()?;
        if !output.status.success() {
            return None;
        }
        let path = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
        if java_works(&path) {
            Some(path)
        } else {
            None
        }
    }
}

#[cfg(windows)]
fn find_java_via_where(program: &str) -> Option<PathBuf> {
    let output = Command::new(program).arg("java").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let path = PathBuf::from(line.trim());
        if java_works(&path) {
            return Some(path);
        }
    }
    None
}

#[cfg(windows)]
fn windows_path_entries() -> Vec<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut parts: Vec<String> = Vec::new();
    if let Ok(path) = std::env::var("PATH") {
        parts.extend(path.split(';').map(|s| s.to_string()));
    }

    let mut merge = |hive: winreg::HKEY, subkey: &str| {
        if let Ok(key) = RegKey::predef(hive).open_subkey(subkey) {
            if let Ok(path) = key.get_value::<String, _>("Path") {
                parts.extend(path.split(';').map(|s| s.to_string()));
            }
        }
    };
    merge(HKEY_LOCAL_MACHINE, r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment");
    merge(HKEY_CURRENT_USER, "Environment");

    parts
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn java_works(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let output = Command::new(path).arg("-version").output();
    match output {
        Ok(out) if out.status.success() || !out.stderr.is_empty() => {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            parse_java_major(&text) >= MIN_JAVA_MAJOR
        }
        _ => false,
    }
}

fn parse_java_major(version_output: &str) -> u32 {
    const MARKER: &str = "version \"";
    for line in version_output.lines() {
        if let Some(idx) = line.find(MARKER) {
            let rest = &line[idx + MARKER.len()..];
            let ver = rest.split('"').next().unwrap_or(rest);
            if let Some(major) = ver.split('.').next() {
                if let Ok(n) = major.parse::<u32>() {
                    return if n == 1 {
                        ver.split('.').nth(1).and_then(|s| s.parse().ok()).unwrap_or(8)
                    } else {
                        n
                    };
                }
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::parse_java_major;

    #[test]
    fn parses_oracle_java_version_line() {
        let out = r#"java version "25.0.3" 2026-04-21 LTS
Java(TM) SE Runtime Environment (build 25.0.3+9-LTS-195)"#;
        assert_eq!(parse_java_major(out), 25);
    }

    #[test]
    fn parses_legacy_version_prefix() {
        let out = "version \"1.8.0_401\"\n";
        assert_eq!(parse_java_major(out), 8);
    }

    #[test]
    fn parses_modern_lts() {
        let out = "openjdk version \"21.0.9\" 2025-10-21\n";
        assert_eq!(parse_java_major(out), 21);
    }
}
