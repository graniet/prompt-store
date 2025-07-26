use aes_gcm::{
    aead::{rand_core::RngCore, Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use argon2::Argon2;
use base64::{engine::general_purpose, Engine as _};
use console::style;
use dialoguer::Password;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use super::storage::AppCtx;
use super::utils::ensure_dir;

const MAGIC_PSWD: &[u8; 4] = b"PSWD";

/// Decrypts the master key using a provided password.
pub fn decrypt_key_with_password(key_data: &[u8], password: &str) -> Result<Vec<u8>, String> {
    if !key_data.starts_with(MAGIC_PSWD) {
        return Err("Key is not password protected.".to_string());
    }
    if key_data.len() < 4 + 16 + 12 {
        return Err("Corrupted password key".to_string());
    }
    let salt = &key_data[4..20];
    let nonce = Nonce::from_slice(&key_data[20..32]);
    let cipher_bytes = &key_data[32..];

    let mut pwd_key = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut pwd_key)
        .map_err(|_| "KDF error".to_string())?;

    let tmp_cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&pwd_key));
    let raw = tmp_cipher
        .decrypt(nonce, cipher_bytes)
        .map_err(|_| "Invalid password".to_string())?;

    if raw.len() != 32 {
        return Err("Corrupted key".to_string());
    }
    Ok(raw)
}

/// Load or create encryption key.
pub fn load_or_generate_key(path: &Path) -> Result<(Vec<u8>, bool), String> {
    if path.exists() {
        let mut buf = Vec::new();
        File::open(path)
            .map_err(|e| format!("Unable to open key: {}", e))?
            .read_to_end(&mut buf)
            .map_err(|e| format!("Unable to read key: {}", e))?;

        if buf.starts_with(MAGIC_PSWD) {
            let password = Password::new()
                .with_prompt("Password")
                .interact()
                .map_err(|e| format!("Password error: {}", e))?;
            let raw = decrypt_key_with_password(&buf, &password)?;
            Ok((raw, true))
        } else {
            if buf.len() != 32 {
                return Err("Invalid key length".to_string());
            }
            Ok((buf, false))
        }
    } else {
        if let Some(parent) = path.parent() {
            ensure_dir(parent)?;
        }
        let key = Aes256Gcm::generate_key(OsRng);
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .map_err(|e| format!("Key write error: {}", e))?;
        f.write_all(&key)
            .map_err(|e| format!("Key write error: {}", e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).ok();
        }
        Ok((key.to_vec(), false))
    }
}

/// Rotate encryption key, optional password protection.
pub fn rotate_key(ctx: &AppCtx, use_password: bool) -> Result<(), String> {
    let mut plain = Vec::new();
    if ctx.workspaces_dir.exists() {
        for entry in
            fs::read_dir(&ctx.workspaces_dir).map_err(|e| format!("Read dir error: {}", e))?
        {
            let ent = entry.map_err(|e| format!("Dir read error: {}", e))?;
            let encoded =
                fs::read_to_string(ent.path()).map_err(|e| format!("Read error: {}", e))?;
            let decoded = general_purpose::STANDARD
                .decode(encoded.trim_end())
                .map_err(|_| "Corrupted data".to_string())?;
            let (nonce_bytes, cipher_bytes) = decoded.split_at(12);
            let plaintext = ctx
                .cipher
                .decrypt(Nonce::from_slice(nonce_bytes), cipher_bytes)
                .map_err(|_| "Decrypt error".to_string())?;
            plain.push((ent.path(), plaintext));
        }
    }

    let new_key = Aes256Gcm::generate_key(OsRng);
    let new_cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&new_key));

    if use_password {
        let password = Password::new()
            .with_prompt("New password")
            .with_confirmation("Confirm password", "Mismatch")
            .interact()
            .map_err(|e| format!("Password error: {}", e))?;
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);

        let mut pwd_key = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), &salt, &mut pwd_key)
            .map_err(|_| "KDF error".to_string())?;

        let tmp_cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&pwd_key));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let cipher_bytes = tmp_cipher
            .encrypt(&nonce, new_key.as_ref())
            .map_err(|_| "Encrypt error".to_string())?;

        let mut out = Vec::with_capacity(4 + 16 + 12 + cipher_bytes.len());
        out.extend_from_slice(MAGIC_PSWD);
        out.extend_from_slice(&salt);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&cipher_bytes);
        fs::write(&ctx.key_path, out).map_err(|e| format!("Key write error: {}", e))?;
    } else {
        fs::write(&ctx.key_path, &new_key).map_err(|e| format!("Key write error: {}", e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&ctx.key_path, fs::Permissions::from_mode(0o600)).ok();
        }
    }

    for (path, plaintext) in plain {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let cipher_bytes = new_cipher
            .encrypt(&nonce, plaintext.as_ref())
            .map_err(|_| "Encrypt error".to_string())?;

        let mut out = Vec::with_capacity(12 + cipher_bytes.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&cipher_bytes);
        let encoded = general_purpose::STANDARD.encode(&out);

        fs::write(path, encoded).map_err(|e| format!("Write error: {}", e))?;
    }

    println!("{}", style("Key rotated").green().bold());
    Ok(())
}
