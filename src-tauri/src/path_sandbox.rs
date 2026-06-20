use std::path::{Path, PathBuf};

// ─── Path Sandboxing ──────────────────────────────────────────────────────
//
// Directories the dashboard is permitted to read/write via the file_* and
// run_command commands. Editable at build time via the BOOTSTREEP_ALLOWED_PATHS
// env (colon-separated). Defaults cover common admin read paths.
//
//   Linux: /home, /opt, /etc, /var, /srv, /tmp, /var/lib/docker
//   macOS: $HOME, /opt/homebrew, /usr/local, /private/var, /tmp
//
// Keep this list small and intentional. Anything not listed is denied.
const ALLOWED_PREFIXES_DEFAULT: &[&str] = &[
    "/home",
    "/opt",
    "/etc",
    "/var",
    "/srv",
    "/tmp",
    "/var/lib/docker",
];

fn allowed_prefixes() -> Vec<String> {
    if let Ok(custom) = std::env::var("BOOTSTREEP_ALLOWED_PATHS") {
        let parts: Vec<String> = custom
            .split(':')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !parts.is_empty() {
            return parts;
        }
    }
    ALLOWED_PREFIXES_DEFAULT.iter().map(|s| s.to_string()).collect()
}

pub fn canonicalize(base_dir: &Path, user_path: &str) -> Result<PathBuf, String> {
    let p = Path::new(user_path);
    let canonical = p
        .canonicalize()
        .map_err(|_| format!("Invalid or non-existent path: {}", user_path))?;

    let prefixes = allowed_prefixes();
    let valid = prefixes.iter().any(|prefix| canonical.starts_with(prefix));
    if !valid {
        return Err(format!(
            "Path {} is outside allowed directories: {:?}",
            user_path, prefixes
        ));
    }

    // Symlink-escape: if the user-supplied path itself is a symlink, follow it
    // and ensure the resolved target is still inside an allowed prefix.
    if p.is_symlink() {
        let target = p
            .read_link()
            .map_err(|_| "Cannot read symlink".to_string())?;
        let resolved_target = if target.is_absolute() {
            target.canonicalize().map_err(|_| "Cannot resolve symlink".to_string())?
        } else {
            base_dir
                .join(&target)
                .canonicalize()
                .map_err(|_| "Symlink escape".to_string())?
        };
        if !prefixes.iter().any(|p| resolved_target.starts_with(p)) {
            return Err("Symlink escape detected".to_string());
        }
    }

    Ok(canonical)
}

pub fn validate_remote_path(user_path: &str) -> Result<String, String> {
    if user_path.contains("..") {
        return Err("Path traversal detected".to_string());
    }
    if user_path.contains('\'') || user_path.contains('"') || user_path.contains('`') {
        return Err("Invalid characters in path".to_string());
    }
    if user_path.len() > 4096 {
        return Err("Path too long".to_string());
    }
    // Reject newlines / control chars (defense in depth even after shell-escape).
    if user_path.chars().any(|c| c.is_control()) {
        return Err("Control characters not allowed in path".to_string());
    }
    Ok(user_path.to_string())
}
