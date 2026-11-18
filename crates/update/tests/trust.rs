use ring::signature::{Ed25519KeyPair, KeyPair};
use wordcraft_update::manifest::{Asset, Manifest, verify_manifest, verify_package};

fn signed() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let key = Ed25519KeyPair::from_seed_unchecked(&[42; 32]).unwrap();
    let manifest = Manifest {
        schema: 1,
        repository: "JeongEEE/wordcraft".into(),
        version: "0.18.3".into(),
        tag: "wordcraft-v0.18.3".into(),
        assets: vec![Asset {
            name: "word-0.18.3-windows-x64.msi".into(),
            size: 3,
            sha256: craft_update::http::hex(ring::digest::digest(&ring::digest::SHA256, b"abc").as_ref()),
        }],
    };
    let bytes = serde_json::to_vec(&manifest).unwrap();
    (bytes.clone(), key.sign(&bytes).as_ref().to_vec(), key.public_key().as_ref().to_vec())
}

#[test]
fn authentic_manifest_binds_repository_tag_version_and_package() {
    let (bytes, sig, key) = signed();
    let manifest = verify_manifest(&bytes, &sig, &key, "0.18.2").unwrap();
    assert_eq!(manifest.version, "0.18.3");
    assert_eq!(manifest.asset("windows-x64-msi").unwrap().name, "word-0.18.3-windows-x64.msi");
    assert!(manifest.asset("macos-universal").is_err());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("package");
    std::fs::write(&path, b"abc").unwrap();
    verify_package(&path, &manifest.assets[0]).unwrap();
    std::fs::write(&path, b"abd").unwrap();
    assert!(verify_package(&path, &manifest.assets[0]).is_err());
}

#[test]
fn altered_unsigned_wrong_key_and_rollback_manifests_are_refused() {
    let (mut bytes, sig, key) = signed();
    assert!(verify_manifest(&bytes, &sig, &[0; 32], "0.18.2").is_err());
    assert!(verify_manifest(&bytes, &[], &key, "0.18.2").is_err());
    assert!(verify_manifest(&bytes, &sig, &key, "0.18.3").is_err());
    assert!(verify_manifest(&bytes, &sig, &key, "0.19.0").is_err());
    bytes.push(b' ');
    assert!(verify_manifest(&bytes, &sig, &key, "0.18.2").is_err());
}

#[test]
fn even_signed_invalid_identity_and_paths_are_refused() {
    let (bytes, _, _) = signed();
    let original: Manifest = serde_json::from_slice(&bytes).unwrap();
    let key = Ed25519KeyPair::from_seed_unchecked(&[42; 32]).unwrap();
    for change in 0..7 {
        let mut m = original.clone();
        match change {
            0 => m.repository = "storytold/wordcraft".into(),
            1 => m.tag = "wordcraft-v0.18.4".into(),
            2 => m.assets[0].name = "../word.exe".into(),
            3 => m.assets[0].size = 0,
            4 => m.assets[0].size = 5 << 30,
            5 => m.assets[0].sha256 = "00".into(),
            _ => m.assets.push(m.assets[0].clone()),
        }
        let bytes = serde_json::to_vec(&m).unwrap();
        assert!(verify_manifest(&bytes, key.sign(&bytes).as_ref(), key.public_key().as_ref(), "0.18.2").is_err(), "case {change}");
    }
}

#[test]
#[ignore = "Requires a signed release directory in WORD_UPDATE_VERIFY_RELEASE"]
fn signed_release_matches_the_embedded_key_and_every_downloaded_package() {
    let dir = std::env::var_os("WORD_UPDATE_VERIFY_RELEASE").map(std::path::PathBuf::from).expect("release directory");
    let bytes = std::fs::read(dir.join("WordUpdate.json")).unwrap();
    let sig = wordcraft_update::manifest::decode_hex(std::fs::read_to_string(dir.join("WordUpdate.sig")).unwrap().trim()).unwrap();
    let manifest = verify_manifest(&bytes, &sig, &wordcraft_update::public_key().unwrap(), "0.0.0").unwrap();
    assert_eq!(manifest.assets.len(), 3);
    for asset in &manifest.assets {
        verify_package(&dir.join(&asset.name), asset).unwrap();
    }
}
