use rand::{distributions::Alphanumeric, Rng};
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

/// Generate a new unique alphanumeric ID.
pub fn new_id(dir: &Path) -> String {
    loop {
        let id: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect::<String>()
            .to_lowercase();

        let path = dir.join(format!("{}.prompt", &id));
        if !path.exists() {
            return id;
        }
    }
}
