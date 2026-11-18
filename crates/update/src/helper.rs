//! The copied launcher finishes an authenticated update after its parent really exits.
use super::{
    APP,
    download::{Prepared, read_small, read_verified},
    manifest,
    target::Target,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FLAG: &str = "--word-update-helper";
const HEALTH_ENV: &str = "WORD_UPDATE_HEALTH_FILE";
const MAX_PLAN: usize = 512 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    parent: u32,
    target: Target,
    #[serde(default)]
    reopen: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Outcome {
    pub success: bool,
    pub version: String,
    pub message: String,
}

fn helper_name() -> &'static str {
    if cfg!(windows) { "word-update-helper.exe" } else { "word-update-helper" }
}

pub(crate) fn launch(prepared: Prepared, reopen: Vec<String>) -> Result<(), String> {
    prepared.target.validate()?;
    validate_reopen(&reopen)?;
    let dir = prepared.folder.path();
    // Authenticate again before creating an executable helper or recording an install plan.
    let verified = read_verified(dir)?;
    manifest::verify_package(&dir.join(&verified.asset(prepared.target.platform())?.name), verified.asset(prepared.target.platform())?)?;
    let helper = dir.join(helper_name());
    std::fs::copy(std::env::current_exe().map_err(|e| e.to_string())?, &helper).map_err(|e| e.to_string())?;
    let plan = Plan { parent: std::process::id(), target: prepared.target, reopen };
    let plan_path = dir.join("install.json");
    let bytes = serde_json::to_vec(&plan).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_PLAN {
        return Err("Word's update restart record is too large.".into());
    }
    std::fs::write(&plan_path, bytes).map_err(|e| e.to_string())?;
    craft_update::process::command(&helper)
        .arg(FLAG)
        .arg(&plan_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not start Word's update helper: {e}"))?;
    // The helper now owns these files. Startup garbage collection removes completed helpers.
    let _ = prepared.folder.keep();
    Ok(())
}

/// Recognized helper invocations never fall through to the normal word processor.
pub fn finish(args: &[std::ffi::OsString]) -> Option<Result<(), String>> {
    if args.first().is_none_or(|a| a != FLAG) {
        return None;
    }
    Some((|| {
        if args.len() != 2 {
            return Err("Invalid Word update helper arguments.".into());
        }
        let plan_path = PathBuf::from(args.get(1).ok_or("Missing Word update plan.")?);
        let plan: Plan = serde_json::from_slice(&read_small(&plan_path, MAX_PLAN)?).map_err(|e| e.to_string())?;
        validate_reopen(&plan.reopen)?;
        let dir = validate_helper(&plan_path, &plan)?;
        let version = read_verified(&dir).map(|m| m.version).unwrap_or_else(|_| env!("CARGO_PKG_VERSION").into());
        let result = apply(&dir, &plan);
        let outcome = Outcome { success: result.is_ok(), version, message: result.as_ref().err().cloned().unwrap_or_default() };
        record(&plan.target, &outcome);
        // Never launch another copy when the parent did not exit or another Word is still open.
        if result.is_err() && !craft_update::process::running(plan.target.running_path()).unwrap_or(true) {
            let _ = restart(&plan.target, None, &plan.reopen);
        }
        result
    })())
}

fn validate_helper(plan_path: &Path, plan: &Plan) -> Result<PathBuf, String> {
    let dir = plan_path.parent().ok_or("No update folder.")?.canonicalize().map_err(|e| e.to_string())?;
    let exe = std::env::current_exe().and_then(std::fs::canonicalize).map_err(|e| e.to_string())?;
    let paths = craft_update::Paths::for_exe(&APP, &plan.target.exe())?;
    let expected = paths.cache.join("word-updates").canonicalize().map_err(|e| e.to_string())?;
    if dir.parent() != Some(expected.as_path())
        || !dir.file_name().is_some_and(|s| s.to_string_lossy().starts_with("word-update-"))
        || exe != dir.join(helper_name())
        || plan_path.file_name().is_none_or(|s| s != "install.json")
        || plan.parent == 0
        || plan.parent == std::process::id()
    {
        return Err("The update helper is outside Word's own update folder.".into());
    }
    plan.target.validate()?;
    Ok(dir)
}

fn apply(dir: &Path, plan: &Plan) -> Result<(), String> {
    let manifest = read_verified(dir)?;
    let asset = manifest.asset(plan.target.platform())?;
    let package = dir.join(&asset.name);
    manifest::verify_package(&package, asset)?;
    if !wait_for_parent(plan.parent, Duration::from_secs(90)) {
        return Err("Word did not exit; no update was installed.".into());
    }
    if craft_update::process::running(plan.target.running_path())? {
        return Err("Another Word window is still running; no update was installed.".into());
    }
    plan.target.validate()?;
    match &plan.target {
        #[cfg(target_os = "macos")]
        Target::Mac { bundle } => super::macos::install(&package, bundle, &manifest.version, dir, &plan.reopen),
        #[cfg(windows)]
        Target::Msi { .. } | Target::Portable { .. } => super::windows::install(&package, &plan.target, &manifest.version, dir, &plan.reopen),
        #[allow(unreachable_patterns)]
        _ => Err("This platform cannot install this Word update.".into()),
    }
}

pub fn wait_for_parent(pid: u32, timeout: Duration) -> bool {
    if pid == 0 || pid == std::process::id() {
        return false;
    }
    #[cfg(windows)]
    {
        craft_update::process::wait_for_exit(pid, timeout)
    }
    #[cfg(unix)]
    {
        use nix::{errno::Errno, sys::signal::kill, unistd::Pid};
        let Ok(pid) = i32::try_from(pid) else { return false };
        let start = crate::instant();
        loop {
            match kill(Pid::from_raw(pid), None) {
                Err(Errno::ESRCH) => return true,
                Err(_) => return false,
                Ok(()) if start.elapsed() >= timeout => return false,
                Ok(()) => std::thread::sleep(Duration::from_millis(100)),
            }
        }
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = timeout;
        false
    }
}

/// Signal only after the first frame has successfully rendered, not merely after spawning.
pub fn acknowledge_startup() {
    static ACKNOWLEDGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if ACKNOWLEDGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let Some(path) = std::env::var_os(HEALTH_ENV).map(PathBuf::from) else { return };
    if path.file_name().is_none_or(|n| n != "healthy")
        || !path.parent().and_then(Path::file_name).is_some_and(|n| n.to_string_lossy().starts_with("word-update-"))
    {
        return;
    }
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().write(true).create_new(true).open(path) {
        let _ = file.write_all(b"Word ready");
    }
}

fn validate_reopen(reopen: &[String]) -> Result<(), String> {
    // Absolute paths cannot become CLI switches, and do not depend on the helper's cwd.
    if reopen.iter().any(|p| !Path::new(p).is_absolute() || p.contains('\0')) {
        return Err("An update restart file must have an absolute path.".into());
    }
    Ok(())
}

fn restart_command(target: &Target, health: Option<&Path>, reopen: &[String]) -> std::process::Command {
    let mut cmd = craft_update::process::command(target.exe());
    cmd.args(reopen);
    cmd.env_remove("CRAFT_UPDATE_DEV")
        .env_remove("WORDCRAFT_CONTROL_KEY")
        .env_remove("WORDCRAFT_CONTROL_PORT")
        .env_remove("ARTCRAFT_LAUNCHER_PRETEND_VERSION")
        .env_remove("WORD_PRETEND_VERSION");
    if let Some(path) = health {
        cmd.env(HEALTH_ENV, path);
    } else {
        cmd.env_remove(HEALTH_ENV);
    }
    cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    cmd
}

pub(crate) fn restart(target: &Target, health: Option<&Path>, reopen: &[String]) -> Result<(), String> {
    validate_reopen(reopen)?;
    let mut child = restart_command(target, health, reopen).spawn().map_err(|e| format!("Could not restart Word: {e}"))?;
    let Some(health) = health else { return Ok(()) };
    let start = crate::instant();
    while start.elapsed() < Duration::from_secs(60) {
        if health.is_file() {
            return Ok(());
        }
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            return Err("The updated Word exited before its first frame.".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
    Err("The updated Word did not finish starting within 60 seconds.".into())
}

pub(crate) fn record(target: &Target, outcome: &Outcome) {
    let Ok(paths) = craft_update::Paths::for_exe(&APP, &target.exe()) else { return };
    if std::fs::create_dir_all(&paths.data).is_err() {
        return;
    }
    if let Ok(bytes) = serde_json::to_vec(outcome) {
        let _ = std::fs::write(paths.data.join("word-update-result.json"), bytes);
    }
}

pub fn take_outcome() -> Option<Outcome> {
    let path = craft_update::Paths::for_app(&APP).ok()?.data.join("word-update-result.json");
    let bytes = read_small(&path, 16 * 1024).ok()?;
    let _ = std::fs::remove_file(path);
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_plan_keeps_the_open_documents_as_literal_arguments() {
        let root = tempfile::tempdir().unwrap();
        let bundle = root.path().join("Word.app");
        let file = root.path().join("한글 보고서 --sample.docx");
        let plan: Plan = serde_json::from_value(serde_json::json!({
            "parent": 123,
            "target": Target::Mac { bundle },
            "reopen": [file],
        }))
        .expect("an update plan must carry its open files");
        let encoded = serde_json::to_value(&plan).unwrap();
        assert_eq!(encoded["reopen"], serde_json::json!([file]));
        let cmd = restart_command(&plan.target, None, &plan.reopen);
        assert_eq!(cmd.get_args().collect::<Vec<_>>(), vec![file.as_os_str()]);
        assert!(validate_reopen(&plan.reopen).is_ok());
        for bad in ["--sample", "relative.docx", ""] {
            assert!(validate_reopen(&[bad.into()]).is_err());
        }
    }

    #[test]
    fn old_update_plans_still_restart_without_files() {
        let root = tempfile::tempdir().unwrap();
        let plan: Plan = serde_json::from_value(serde_json::json!({
            "parent": 123,
            "target": Target::Mac { bundle: root.path().join("Word.app") },
        }))
        .unwrap();
        assert!(restart_command(&plan.target, None, &plan.reopen).get_args().next().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn restarted_process_receives_each_unicode_file_without_shell_splitting() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let target = Target::Mac { bundle: root.path().join("Word.app") };
        let exe = target.exe();
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$0.args\"\nif [ -n \"$WORD_UPDATE_HEALTH_FILE\" ]; then printf ready > \"$WORD_UPDATE_HEALTH_FILE\"; fi\n").unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o700)).unwrap();
        let first = root.path().join("한글 보고서.docx").to_str().unwrap().to_string();
        let second = root.path().join("제목 $(literal) 'quotes'.docx").to_str().unwrap().to_string();
        let health = root.path().join("healthy");
        restart(&target, Some(&health), &[first.clone(), second.clone()]).unwrap();
        let received = std::fs::read_to_string(exe.with_file_name("Word.args")).unwrap();
        assert_eq!(received, format!("{first}\n{second}\n"));
    }
    #[test]
    fn never_waits_for_itself_or_pid_zero() {
        assert!(!wait_for_parent(std::process::id(), Duration::ZERO));
        assert!(!wait_for_parent(0, Duration::ZERO));
    }
    #[cfg(unix)]
    #[test]
    fn live_parent_times_out_and_exited_parent_is_accepted() {
        let mut child = std::process::Command::new("/bin/sleep").arg("5").spawn().unwrap();
        assert!(!wait_for_parent(child.id(), Duration::from_millis(20)));
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(wait_for_parent(child.id(), Duration::from_millis(20)));
    }
}
