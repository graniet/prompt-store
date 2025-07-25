use crate::core::{crypto::rotate_key, storage::AppCtx};

/// Rotate the encryption key.
pub fn run(ctx: &AppCtx, use_password: bool) -> Result<(), String> {
    rotate_key(ctx, use_password)
}
