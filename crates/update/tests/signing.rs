use ring::signature::{Ed25519KeyPair, KeyPair};

#[test]
fn release_signing_requires_all_platforms_and_matching_pinned_public_key() {
    let dir = tempfile::tempdir().unwrap();
    let key = Ed25519KeyPair::from_seed_unchecked(&[42; 32]).unwrap();
    let key_file = dir.path().join("public.hex");
    std::fs::write(&key_file, craft_update::http::hex(key.public_key().as_ref())).unwrap();
    let run = || {
        std::process::Command::new(env!("CARGO_BIN_EXE_word-update-sign"))
            .arg("sign")
            .arg(dir.path())
            .arg("0.18.3")
            .arg(&key_file)
            .env("WORD_UPDATE_PRIVATE_KEY", "2a".repeat(32))
            .output()
            .unwrap()
    };
    assert!(!run().status.success(), "missing packages cannot produce an update feed");
    for suffix in ["macos-universal.dmg", "windows-x64.msi", "windows-x64-portable.zip"] {
        std::fs::write(dir.path().join(format!("word-0.18.3-{suffix}")), format!("payload {suffix}")).unwrap();
    }
    let answer = run();
    assert!(answer.status.success(), "{}", String::from_utf8_lossy(&answer.stderr));
    let bytes = std::fs::read(dir.path().join("WordUpdate.json")).unwrap();
    let signature = wordcraft_update::manifest::decode_hex(std::fs::read_to_string(dir.path().join("WordUpdate.sig")).unwrap().trim()).unwrap();
    let verified = wordcraft_update::manifest::verify_manifest(&bytes, &signature, key.public_key().as_ref(), "0.18.2").unwrap();
    assert_eq!(verified.assets.len(), 3);
    for asset in &verified.assets {
        wordcraft_update::manifest::verify_package(&dir.path().join(&asset.name), asset).unwrap();
    }
    std::fs::write(&key_file, "00".repeat(32)).unwrap();
    assert!(!run().status.success(), "a CI secret for another key must not sign a feed");
}

#[test]
fn key_generation_is_private_and_never_overwrites_an_existing_key() {
    let root = tempfile::tempdir().unwrap();
    let private = root.path().join("private.hex");
    let public = root.path().join("public.hex");
    let run = || std::process::Command::new(env!("CARGO_BIN_EXE_word-update-sign")).arg("keygen").arg(&private).arg(&public).output().unwrap();
    let first = run();
    assert!(first.status.success());
    assert!(first.stdout.is_empty(), "the key must not be printed");
    let original = std::fs::read(&private).unwrap();
    let secret = wordcraft_update::manifest::decode_hex(std::str::from_utf8(&original).unwrap()).unwrap();
    let pair = Ed25519KeyPair::from_seed_unchecked(&secret).unwrap();
    let saved_public = wordcraft_update::manifest::decode_hex(std::fs::read_to_string(&public).unwrap().trim()).unwrap();
    assert_eq!(pair.public_key().as_ref(), saved_public);
    assert!(!run().status.success());
    assert_eq!(std::fs::read(&private).unwrap(), original);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(private).unwrap().permissions().mode() & 0o777, 0o600);
    }
}
