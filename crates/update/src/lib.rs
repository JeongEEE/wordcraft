//! Authenticated update packages for Word, independent of the upstream publisher.
#![cfg(not(target_arch = "wasm32"))]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[allow(clippy::disallowed_methods)] // This crate is used only by native desktop targets.
pub fn instant() -> std::time::Instant {
    std::time::Instant::now()
}

pub mod manifest;

pub mod download;
pub mod helper;
pub mod install;
#[cfg(target_os = "macos")]
mod macos;
pub mod target;
#[cfg(windows)]
mod windows;

pub const APP: craft_update::App = craft_update::App {
    developer: "JeongEEE",
    bundle_id_override: Some("io.github.jeongeee.word"),
    ..craft_update::App::new("word", "Word", "Writing and document design", manifest::REPOSITORY)
};

/// Release builds pin this key in source. No environment variable may change an installed
/// application's trust root. Empty means fail closed until the owner provisions their key.
pub fn public_key() -> Result<Vec<u8>, String> {
    pinned_key(include_str!("../public-key.hex"))
}

fn pinned_key(text: &str) -> Result<Vec<u8>, String> {
    let key = manifest::decode_hex(text.trim())?;
    if key.len() != 32 {
        return Err("Word's update signing key has not been configured.".into());
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    #[test]
    fn empty_or_malformed_pinned_key_is_an_error_not_a_trust_root() {
        for bad in ["", "\n", "zz".repeat(32).as_str(), "ab".repeat(31).as_str(), "ab".repeat(33).as_str(), "abc"] {
            assert!(super::pinned_key(bad).is_err(), "{bad:?}");
        }
        assert_eq!(super::pinned_key(&format!("{}\n", "ab".repeat(32))).unwrap(), vec![0xab; 32]);
    }

    /// Until the owner pins Word's release key (crates/update/public-key.hex), every update is
    /// refused. Replace this with a test pinning that key's value once it is committed.
    #[test]
    fn unprovisioned_release_key_refuses_updates() {
        assert!(super::public_key().is_err());
    }
}
