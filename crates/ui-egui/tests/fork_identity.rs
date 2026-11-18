//! Word, the personal fork, must never present itself as, or update from, upstream WordCraft.
#![cfg(not(target_arch = "wasm32"))]

use wordcraft_ui_egui::UPDATE_APP;

#[test]
fn fork_app_identity_is_word() {
    assert_eq!(UPDATE_APP.id, "word");
    assert_eq!(UPDATE_APP.name, "Word");
    assert_eq!(UPDATE_APP.developer, "JeongEEE");
    assert_eq!(UPDATE_APP.bundle_id(), "io.github.jeongeee.word");
    assert_ne!(UPDATE_APP.id, craft_update::suite::WORDCRAFT.id);
    assert_ne!(UPDATE_APP.bundle_id(), craft_update::suite::WORDCRAFT.bundle_id());
}

#[test]
fn fork_never_checks_upstream_releases() {
    assert_eq!(UPDATE_APP.repo, "JeongEEE/wordcraft");
    assert_ne!(UPDATE_APP.repo, craft_update::suite::WORDCRAFT.repo);
}

#[test]
fn fork_update_folders_are_separate_from_upstream() {
    // craft-update derives its per-user data and cache folders from the display name. The app's
    // own settings folder is checked in `wordcraft-control-key`.
    let paths = craft_update::paths::Paths::for_app(&UPDATE_APP).expect("per-user folders");
    let folder = if cfg!(any(target_os = "macos", target_os = "windows")) { "Word" } else { "word" };
    assert!(paths.data.ends_with(folder), "{}", paths.data.display());
    assert!(paths.cache.ends_with(folder), "{}", paths.cache.display());
}
