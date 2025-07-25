use std::fs;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Ensure directory exists.
pub fn ensure_dir(path: &Path) -> Result<(), String> {
    if !path.exists() {
        fs::create_dir_all(path)
            .map_err(|e| format!("Unable to create directory {}: {}", path.display(), e))?;
    }
    #[cfg(unix)]
    {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).ok();
    }
    Ok(())
}

/// Compute next numeric ID.
pub fn next_id(dir: &Path) -> Result<u64, String> {
    let mut max_id = 0;
    if dir.exists() {
        for entry in fs::read_dir(dir).map_err(|e| format!("Unable to read dir: {}", e))? {
            let ent = entry.map_err(|e| format!("Dir read error: {}", e))?;
            if let Some(name) = ent.file_name().to_str() {
                if let Ok(n) = name.split('.').next().unwrap_or("").parse::<u64>() {
                    if n > max_id {
                        max_id = n;
                    }
                }
            }
        }
    }
    Ok(max_id + 1)
}
