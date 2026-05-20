use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::fs;
use std::path::Path;
use thiserror::Error;

const KEY_FILE: &str = "data/server.key";

#[derive(Debug, Error)]
pub enum KeystoreError {
    #[error("keystore I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid key length: expected 32 bytes, got {0}")]
    InvalidKeyLength(usize),
}

/// Whether the returned key was loaded from disk or freshly generated.
pub enum KeyOrigin {
    Loaded,
    Generated,
}

/// Load the server signing key from `data/server.key`, or generate one and
/// persist it (with `0600` permissions on Unix) if the file does not exist.
pub fn load_or_generate_keypair() -> Result<(SigningKey, KeyOrigin), KeystoreError> {
    let path = Path::new(KEY_FILE);
    if path.exists() {
        load_keypair(path).map(|k| (k, KeyOrigin::Loaded))
    } else {
        generate_and_save_keypair(path).map(|k| (k, KeyOrigin::Generated))
    }
}

fn load_keypair(path: &Path) -> Result<SigningKey, KeystoreError> {
    let bytes = fs::read(path)?;
    let len = bytes.len();
    let seed: [u8; 32] = bytes
        .try_into()
        .map_err(|_| KeystoreError::InvalidKeyLength(len))?;
    Ok(SigningKey::from_bytes(&seed))
}

fn generate_and_save_keypair(path: &Path) -> Result<SigningKey, KeystoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let keypair = SigningKey::generate(&mut OsRng);
    fs::write(path, keypair.to_bytes())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms)?;
    }

    Ok(keypair)
}
