//! Filesystem transactions. A failed replacement or restart restores the previous payload.
use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::Path;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn regular(path: &Path) -> Result<bool, String> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.is_file() => Ok(true),
        Ok(_) => Err(format!("Refusing to replace a link or directory: {}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(err(e)),
    }
}

fn safe_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or_default().to_ascii_uppercase();
    !name.is_empty()
        && name.len() <= 160
        && name.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        && !name.starts_with('.')
        && !name.ends_with('.')
        && !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && stem.ends_with(|c: char| c.is_ascii_digit()))
}

fn shipped_name(name: &str) -> bool {
    matches!(name, "word.exe" | "word-cli.exe" | "README.md" | "LICENSE" | "LICENSE-MIT" | "LICENSE-APACHE" | "NOTICE" | "ATTRIBUTION.md")
        || (name.starts_with("OFL-") && name.ends_with(".txt"))
}

pub fn extract_portable(package: &Path, output: &Path, version: &str) -> Result<(), String> {
    let file = std::fs::File::open(package).map_err(err)?;
    let mut zip = zip::ZipArchive::new(file).map_err(err)?;
    if zip.len() > 256 {
        return Err("Too many files in the Word update.".into());
    }
    let root = format!("word-{version}-windows-x64-portable/");
    let mut names = HashSet::new();
    let mut files = Vec::new();
    let mut total = 0u64;
    // Validate the entire archive before creating any output. Do not use ZipArchive::extract:
    // Windows treats backslashes, device names and alternate streams as special paths too.
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(err)?;
        let full = entry.name().map_err(err)?;
        if entry.is_dir() && full == root {
            continue;
        }
        let name = full.strip_prefix(&root).ok_or("Unexpected update archive root.")?;
        total = total.checked_add(entry.size()).ok_or("Update archive is too large.")?;
        if !safe_name(name)
            || !shipped_name(name)
            || !names.insert(name.to_ascii_lowercase())
            || entry.is_dir()
            || entry.unix_mode().is_some_and(|mode| mode & 0o170000 == 0o120000)
            || total > super::manifest::MAX_PACKAGE
        {
            return Err("Unsafe or duplicate file in the Word update archive.".into());
        }
        files.push((i, name.to_string(), entry.size()));
    }
    if !names.contains("word.exe") || !names.contains("word-cli.exe") {
        return Err("The update archive is missing Word programs.".into());
    }
    std::fs::create_dir(output).map_err(err)?;
    for (i, name, size) in files {
        let mut entry = zip.by_index(i).map_err(err)?;
        let mut out = std::fs::OpenOptions::new().write(true).create_new(true).open(output.join(name)).map_err(err)?;
        let copied = std::io::copy(&mut Read::take(&mut entry, size.saturating_add(1)), &mut out).map_err(err)?;
        if copied != size {
            return Err("The extracted update size is invalid.".into());
        }
        out.flush().map_err(err)?;
    }
    Ok(())
}

pub fn replace_portable(staged: &Path, target: &Path, restart: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(staged).map_err(err)? {
        let entry = entry.map_err(err)?;
        let name = entry.file_name().into_string().map_err(|_| "Non-UTF8 update filename.")?;
        if !safe_name(&name) || !shipped_name(&name) || !regular(&entry.path())? {
            return Err("Unsafe staged update file.".into());
        }
        regular(&target.join(&name))?;
        names.push(name);
    }
    if !names.iter().any(|s| s == "word.exe") || !names.iter().any(|s| s == "word-cli.exe") {
        return Err("Missing Word programs.".into());
    }
    names.sort();
    let backup = tempfile::Builder::new().prefix(".word-update-backup-").tempdir_in(target).map_err(err)?;
    let mut changed = Vec::new();
    let result = (|| {
        for name in &names {
            let dest = target.join(name);
            let existed = regular(&dest)?;
            if existed {
                std::fs::rename(&dest, backup.path().join(name)).map_err(err)?;
            }
            changed.push((name.clone(), existed));
            std::fs::rename(staged.join(name), &dest).map_err(err)?;
        }
        restart()
    })();
    if let Err(reason) = result {
        let mut failures = Vec::new();
        for (name, existed) in changed.into_iter().rev() {
            let dest = target.join(&name);
            if dest.exists()
                && let Err(e) = std::fs::remove_file(&dest)
            {
                failures.push(err(e));
            }
            if existed && let Err(e) = std::fs::rename(backup.path().join(&name), &dest) {
                failures.push(err(e));
            }
        }
        if !failures.is_empty() {
            return Err(format!("{reason}; recovery files kept at {}: {}", backup.keep().display(), failures.join(", ")));
        }
        return Err(reason);
    }
    Ok(())
}

pub fn replace_bundle(staged: &Path, target: &Path, restart: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    for path in [staged, target] {
        if !std::fs::symlink_metadata(path).map_err(err)?.is_dir() {
            return Err("An update bundle is not a regular directory.".into());
        }
    }
    let parent = target.parent().ok_or("The app has no parent folder.")?;
    let backup = tempfile::Builder::new().prefix(".word-update-backup-").tempdir_in(parent).map_err(err)?;
    let old = backup.path().join("Word.app");
    std::fs::rename(target, &old).map_err(err)?;
    let result = std::fs::rename(staged, target).map_err(err).and_then(|()| restart());
    if let Err(reason) = result {
        if target.exists()
            && let Err(e) = std::fs::rename(target, backup.path().join("failed.app"))
        {
            return Err(format!("{reason}; recovery files kept at {} ({e})", backup.keep().display()));
        }
        if let Err(e) = std::fs::rename(&old, target) {
            return Err(format!("{reason}; recovery files kept at {} ({e})", backup.keep().display()));
        }
        return Err(reason);
    }
    Ok(())
}
