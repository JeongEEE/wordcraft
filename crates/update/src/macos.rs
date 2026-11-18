//! Preserve bundle signatures and atomically replace only this fork's installed application.
use super::{APP, helper, install::replace_bundle, target::Target};
use std::path::Path;
use std::time::Duration;

fn run(program: &str, args: &[&std::ffi::OsStr]) -> Result<String, String> {
    let mut cmd = craft_update::process::command(program);
    cmd.args(args);
    craft_update::process::run_command(cmd, program, Duration::from_secs(120))
}

pub(crate) fn install(package: &Path, bundle: &Path, version: &str, work: &Path, reopen: &[String]) -> Result<(), String> {
    let target = Target::Mac { bundle: bundle.to_path_buf() };
    install_restarting(package, bundle, version, work, || helper::restart(&target, Some(&work.join("healthy")), reopen))
}

fn install_restarting(package: &Path, bundle: &Path, version: &str, work: &Path, restart: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    let old = craft_update::detect::macos::read_bundle(&APP, bundle).and_then(|b| b.version).ok_or("Cannot read installed Word version.")?;
    let new = craft_update::Version::parse(version).map_err(|e| e.to_string())?;
    if new <= old {
        return Err("The installed Word is already this version or newer.".into());
    }
    let mount = tempfile::Builder::new().prefix("word-mount-").tempdir_in(work).map_err(|e| e.to_string())?;
    if let Err(reason) = run(
        "/usr/bin/hdiutil",
        &["attach".as_ref(), "-readonly".as_ref(), "-nobrowse".as_ref(), "-mountpoint".as_ref(), mount.path().as_os_str(), package.as_os_str()],
    ) {
        let _ = mount.keep();
        return Err(reason);
    }
    let result = stage_and_replace(mount.path(), bundle, version, restart);
    let detached = run("/usr/bin/hdiutil", &["detach".as_ref(), mount.path().as_os_str()]);
    // A mounted disk must never be recursively removed by TempDir cleanup on failure.
    if let Err(reason) = detached {
        let _ = mount.keep();
        return result.and(Err(reason));
    }
    result
}

fn stage_and_replace(mount: &Path, bundle: &Path, version: &str, restart: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    let source = mount.join("Word.app");
    verify_bundle(&source, version)?;
    let sibling =
        tempfile::Builder::new().prefix(".word-update-stage-").tempdir_in(bundle.parent().ok_or("No app parent.")?).map_err(|e| e.to_string())?;
    let staged = sibling.path().join("Word.app");
    run("/usr/bin/ditto", &[source.as_os_str(), staged.as_os_str()])?;
    verify_bundle(&staged, version)?;
    replace_bundle(&staged, bundle, restart)
}

fn verify_bundle(bundle: &Path, version: &str) -> Result<(), String> {
    let installed = craft_update::detect::macos::read_bundle(&APP, bundle).ok_or("The downloaded bundle is not Word.")?;
    if installed.version.map(|v| v.to_string()).as_deref() != Some(version) || !bundle.join("Contents/MacOS/Word").is_file() {
        return Err("The downloaded Word bundle has the wrong version or executable.".into());
    }
    run("/usr/bin/codesign", &["--verify".as_ref(), "--deep".as_ref(), "--strict".as_ref(), bundle.as_os_str()])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires a packaged Word DMG in WORD_UPDATE_SMOKE_PACKAGE"]
    fn native_dmg_roundtrip_and_failed_restart_restore_the_old_bundle() {
        let package = std::env::var_os("WORD_UPDATE_SMOKE_PACKAGE").map(std::path::PathBuf::from).expect("DMG path");
        let root = tempfile::tempdir().unwrap();
        let bundle = root.path().join("Word.app");
        std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        std::fs::write(bundle.join("Contents/Info.plist"), r#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>io.github.jeongeee.word</string><key>CFBundleShortVersionString</key><string>0.0.1</string></dict></plist>"#).unwrap();
        std::fs::write(bundle.join("Contents/MacOS/Word"), "old payload").unwrap();
        let failed = install_restarting(&package, &bundle, env!("CARGO_PKG_VERSION"), root.path(), || {
            assert!(run(bundle.join("Contents/MacOS/Word").to_str().unwrap(), &["--version".as_ref()]).unwrap().contains(env!("CARGO_PKG_VERSION")));
            Err("Simulated native restart failure".into())
        });
        assert!(failed.is_err());
        assert_eq!(std::fs::read_to_string(bundle.join("Contents/MacOS/Word")).unwrap(), "old payload");
        install_restarting(&package, &bundle, env!("CARGO_PKG_VERSION"), root.path(), || {
            let output = run(bundle.join("Contents/MacOS/Word").to_str().unwrap(), &["--version".as_ref()])?;
            assert!(output.contains(env!("CARGO_PKG_VERSION")));
            Ok(())
        })
        .unwrap();
        verify_bundle(&bundle, env!("CARGO_PKG_VERSION")).unwrap();
    }
}
