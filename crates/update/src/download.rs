//! Background download. Nothing in this module changes the installed app.
use super::{
    APP,
    manifest::{self, Manifest},
    target::Target,
};
use craft_update::{Http, Release, Stop, http::Transfer};
use std::path::{Path, PathBuf};

pub struct Prepared {
    pub version: String,
    pub(crate) folder: tempfile::TempDir,
    pub(crate) target: Target,
}

pub fn release_url(tag: &str, name: &str) -> Result<String, String> {
    craft_update::release::expected_download_url(manifest::REPOSITORY, tag, name).ok_or_else(|| "Invalid Word release path.".into())
}

impl Prepared {
    pub fn download(release: &Release, target: Target, stop: &Stop, progress: &mut dyn FnMut(Transfer)) -> Result<Self, String> {
        target.validate()?;
        let key = super::public_key()?;
        if release.repo != manifest::REPOSITORY || release.prerelease {
            return Err("The update is not a stable Word fork release.".into());
        }
        let http = Http::new("Word/Update")?;
        let bytes = http
            .get_bytes(&release_url(&release.tag, manifest::MANIFEST_NAME)?, manifest::MAX_MANIFEST)?
            .ok_or("This release has no signed Word update manifest.")?;
        let sig = http.get_bytes(&release_url(&release.tag, manifest::SIGNATURE_NAME)?, 256)?.ok_or("This release has no Word update signature.")?;
        let signature = manifest::decode_hex(std::str::from_utf8(&sig).map_err(|e| e.to_string())?.trim())?;
        let verified = manifest::verify_manifest(&bytes, &signature, &key, env!("CARGO_PKG_VERSION"))?;
        if verified.tag != release.tag {
            return Err("The update manifest belongs to another release.".into());
        }
        let asset = verified.asset(target.platform())?;
        let cache = craft_update::Paths::for_app(&APP)?.cache.join("word-updates");
        std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
        let folder = tempfile::Builder::new().prefix("word-update-").tempdir_in(cache).map_err(|e| e.to_string())?;
        let package = folder.path().join(&asset.name);
        http.download(&release_url(&verified.tag, &asset.name)?, &package, asset.size, &asset.sha256, stop, progress)?;
        manifest::verify_package(&package, asset)?;
        std::fs::write(folder.path().join(manifest::MANIFEST_NAME), bytes).map_err(|e| e.to_string())?;
        std::fs::write(folder.path().join(manifest::SIGNATURE_NAME), sig).map_err(|e| e.to_string())?;
        Ok(Self { version: verified.version, folder, target })
    }

    /// Called only at actual app exit after the user requested restart and settled all drafts.
    pub fn launch(self, reopen: Vec<String>) -> Result<(), String> {
        let target = self.target.clone();
        let version = self.version.clone();
        let result = super::helper::launch(self, reopen);
        if let Err(reason) = &result {
            super::helper::record(&target, &super::helper::Outcome { success: false, version, message: reason.clone() });
        }
        result
    }
}

pub(crate) fn read_verified(folder: &Path) -> Result<Manifest, String> {
    let bytes = read_small(&folder.join(manifest::MANIFEST_NAME), manifest::MAX_MANIFEST)?;
    let sig = read_small(&folder.join(manifest::SIGNATURE_NAME), 256)?;
    let sig = manifest::decode_hex(std::str::from_utf8(&sig).map_err(|e| e.to_string())?.trim())?;
    manifest::verify_manifest(&bytes, &sig, &super::public_key()?, env!("CARGO_PKG_VERSION"))
}

pub(crate) fn read_small(path: &PathBuf, limit: usize) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("An update record exceeds its size limit.".into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::manifest::{self, Asset, Manifest};
    use ring::signature::{Ed25519KeyPair, KeyPair};

    /// The helper re-reads the saved manifest before installing: without the pinned key (empty
    /// until provisioned, or any other key) even a well-formed signed update must be refused.
    #[test]
    fn helper_refuses_an_update_not_signed_by_the_pinned_key() {
        let key = Ed25519KeyPair::from_seed_unchecked(&[42; 32]).unwrap();
        let m = Manifest {
            schema: 1,
            repository: manifest::REPOSITORY.into(),
            version: "99.0.0".into(),
            tag: "wordcraft-v99.0.0".into(),
            assets: vec![Asset { name: "word-99.0.0-macos-universal.dmg".into(), size: 3, sha256: "ab".repeat(32) }],
        };
        let bytes = serde_json::to_vec(&m).unwrap();
        let sig = key.sign(&bytes);
        assert!(manifest::verify_manifest(&bytes, sig.as_ref(), key.public_key().as_ref(), env!("CARGO_PKG_VERSION")).is_ok());
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(manifest::MANIFEST_NAME), &bytes).unwrap();
        std::fs::write(dir.path().join(manifest::SIGNATURE_NAME), craft_update::http::hex(sig.as_ref())).unwrap();
        assert!(super::read_verified(dir.path()).is_err());
    }
}
