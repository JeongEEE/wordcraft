//! The signed, bounded contract between the release job and the installed application.
use std::collections::HashSet;
use std::io::Read;
use std::path::Path;

use ring::{digest, signature};
use serde::{Deserialize, Serialize};

pub const REPOSITORY: &str = "JeongEEE/wordcraft";
pub const MAX_MANIFEST: usize = 32 * 1024;
pub const MAX_PACKAGE: u64 = 2 << 30;
pub const MANIFEST_NAME: &str = "WordUpdate.json";
pub const SIGNATURE_NAME: &str = "WordUpdate.sig";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub repository: String,
    pub version: String,
    pub tag: String,
    pub assets: Vec<Asset>,
}

impl Manifest {
    pub fn asset(&self, platform: &str) -> Result<&Asset, String> {
        let suffix = match platform {
            "macos-universal" => "macos-universal.dmg",
            "windows-x64-msi" => "windows-x64.msi",
            "windows-x64-portable" => "windows-x64-portable.zip",
            _ => return Err("This platform has no Word automatic update package.".into()),
        };
        let name = format!("word-{}-{suffix}", self.version);
        self.assets.iter().find(|a| a.name == name).ok_or_else(|| format!("The signed update has no {platform} package."))
    }

    pub fn validate(&self) -> Result<(), String> {
        let version = craft_update::Version::parse(&self.version).map_err(|e| e.to_string())?;
        if self.schema != 1
            || self.repository != REPOSITORY
            || !version.pre.is_empty()
            || !version.build.is_empty()
            || version.to_string() != self.version
            || self.tag != format!("wordcraft-v{}", self.version)
            || self.assets.is_empty()
            || self.assets.len() > 4
        {
            return Err("The signed update identity or version is invalid.".into());
        }
        let names = ["macos-universal.dmg", "windows-x64.msi", "windows-x64-portable.zip"].map(|suffix| format!("word-{}-{suffix}", self.version));
        let mut seen = HashSet::new();
        for a in &self.assets {
            if !names.contains(&a.name)
                || !seen.insert(&a.name)
                || a.size == 0
                || a.size > MAX_PACKAGE
                || a.sha256.len() != 64
                || !a.sha256.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            {
                return Err("The signed update has an invalid or duplicate package.".into());
            }
        }
        Ok(())
    }
}

pub fn verify_manifest(bytes: &[u8], signature: &[u8], key: &[u8], current: &str) -> Result<Manifest, String> {
    if bytes.len() > MAX_MANIFEST || key.len() != 32 || signature.len() != 64 {
        return Err("The update signature or manifest size is invalid.".into());
    }
    signature::UnparsedPublicKey::new(&signature::ED25519, key)
        .verify(bytes, signature)
        .map_err(|_| "The update was not signed by Word's release key.".to_string())?;
    let manifest: Manifest = serde_json::from_slice(bytes).map_err(|e| format!("Invalid signed update: {e}"))?;
    manifest.validate()?;
    let running = craft_update::Version::parse(current).map_err(|e| e.to_string())?;
    let offered = craft_update::Version::parse(&manifest.version).map_err(|e| e.to_string())?;
    if offered <= running {
        return Err("The update must be newer than the installed version.".into());
    }
    Ok(manifest)
}

pub fn verify_package(path: &Path, asset: &Asset) -> Result<(), String> {
    let info = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !info.is_file() || info.len() != asset.size || info.len() > MAX_PACKAGE {
        return Err("The update package size changed or is not a regular file.".into());
    }
    if package_hash(path)? != asset.sha256 {
        return Err("The update package checksum does not match its signature.".into());
    }
    Ok(())
}

pub fn package_hash(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = digest::Context::new(&digest::SHA256);
    let mut buffer = [0; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(craft_update::http::hex(hash.finish().as_ref()))
}

pub fn decode_hex(text: &str) -> Result<Vec<u8>, String> {
    if text.len() > MAX_MANIFEST || !text.len().is_multiple_of(2) || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid hexadecimal key or signature.".into());
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let hex = std::str::from_utf8(pair).map_err(|e| e.to_string())?;
            u8::from_str_radix(hex, 16).map_err(|e| e.to_string())
        })
        .collect()
}
