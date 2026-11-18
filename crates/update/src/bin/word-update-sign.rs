//! Offline release signing. Private key bytes never appear in command arguments or output.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
#[cfg(not(target_arch = "wasm32"))]
mod native {
    use ring::{
        rand::{SecureRandom, SystemRandom},
        signature::{Ed25519KeyPair, KeyPair},
    };
    use std::path::{Path, PathBuf};
    use wordcraft_update::manifest::{self, Asset, Manifest};

    fn key(seed: &[u8]) -> Result<Ed25519KeyPair, String> {
        Ed25519KeyPair::from_seed_unchecked(seed).map_err(|_| "The signing key must contain exactly 32 random bytes.".into())
    }

    fn write_new(path: &Path, data: &[u8], private: bool) -> Result<(), String> {
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(if private { 0o600 } else { 0o644 });
        }
        #[cfg(not(unix))]
        let _ = private;
        let mut file = options.open(path).map_err(|e| format!("Cannot create {}: {e}", path.display()))?;
        file.write_all(data).and_then(|()| file.sync_all()).map_err(|e| e.to_string())
    }

    fn generate(private: &Path, public: &Path) -> Result<(), String> {
        if private.exists() || public.exists() || private == public {
            return Err("Key generation never overwrites an existing file.".into());
        }
        let mut seed = [0; 32];
        SystemRandom::new().fill(&mut seed).map_err(|_| "The system random generator failed.")?;
        let pair = key(&seed)?;
        write_new(private, craft_update::http::hex(&seed).as_bytes(), true)?;
        write_new(public, craft_update::http::hex(pair.public_key().as_ref()).as_bytes(), false)
    }

    fn sign(folder: &Path, version: &str, public: &Path) -> Result<(), String> {
        let secret = std::env::var("WORD_UPDATE_PRIVATE_KEY").map_err(|_| "WORD_UPDATE_PRIVATE_KEY is not configured.")?;
        let pair = key(&manifest::decode_hex(secret.trim())?)?;
        let expected = manifest::decode_hex(std::fs::read_to_string(public).map_err(|e| e.to_string())?.trim())?;
        if pair.public_key().as_ref() != expected {
            return Err("The signing secret does not match Word's pinned public key.".into());
        }
        let mut assets = Vec::new();
        for suffix in ["macos-universal.dmg", "windows-x64.msi", "windows-x64-portable.zip"] {
            let name = format!("word-{version}-{suffix}");
            let path = folder.join(&name);
            let size = std::fs::symlink_metadata(&path).map_err(|e| format!("Missing {name}: {e}"))?;
            if !size.is_file() {
                return Err(format!("{name} is not a regular file."));
            }
            assets.push(Asset { name, size: size.len(), sha256: manifest::package_hash(&path)? });
        }
        let manifest =
            Manifest { schema: 1, repository: manifest::REPOSITORY.into(), version: version.into(), tag: format!("wordcraft-v{version}"), assets };
        manifest.validate()?;
        let bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
        if bytes.len() > manifest::MAX_MANIFEST {
            return Err("Release manifest is too large.".into());
        }
        let signature = craft_update::http::hex(pair.sign(&bytes).as_ref());
        std::fs::write(folder.join(manifest::MANIFEST_NAME), bytes).map_err(|e| e.to_string())?;
        std::fs::write(folder.join(manifest::SIGNATURE_NAME), signature).map_err(|e| e.to_string())
    }

    fn run() -> Result<(), String> {
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        match args.as_slice() {
            [command, private, public] if command == "keygen" => generate(&PathBuf::from(private), &PathBuf::from(public)),
            [command, folder, version, public] if command == "sign" => {
                sign(&PathBuf::from(folder), version.to_str().ok_or("Invalid version encoding.")?, &PathBuf::from(public))
            }
            _ => Err("Usage: word-update-sign keygen PRIVATE_FILE PUBLIC_FILE | sign RELEASE_DIR VERSION PUBLIC_FILE".into()),
        }
    }

    pub fn main() {
        if let Err(reason) = run() {
            eprintln!("Word release signing: {reason}");
            std::process::exit(1);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    native::main();
}

// `xtask wasm` checks every L0 library and its binaries. Release signing is desktop tooling.
#[cfg(target_arch = "wasm32")]
fn main() {}
