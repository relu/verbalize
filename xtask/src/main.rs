//! `cargo xtask cldr-update <tag> | gen | check-data`
//!
//! Vendors the CLDR JSON the library needs and compiles it
//! into `verbalize/src/data/`. Both the JSON and the generated Rust are
//! checked in; `check-data` is the CI guard against stale output.

mod gen;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const RAW: &str = "https://raw.githubusercontent.com/unicode-org/cldr-json";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["cldr-update", tag] => update(tag),
        ["gen"] => generate(),
        ["check-data"] => check(),
        _ => {
            eprintln!("usage: cargo xtask cldr-update <tag> | gen | check-data");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn cldr_dir() -> PathBuf {
    root().join("xtask/cldr")
}

fn lock_path() -> PathBuf {
    root().join("xtask/cldr.lock")
}

fn data_dir() -> PathBuf {
    root().join("verbalize/src/data")
}

/// Downloads the CLDR files listed in `gen::lang_files` for every supported
/// language plus the upstream LICENSE into `xtask/cldr/`, replacing whatever
/// was there, and records the tag in `xtask/cldr.lock`. The download lands in
/// a sibling directory first, so a failed download leaves the vendored data
/// as it was.
fn update(tag: &str) -> gen::Result<()> {
    let dir = cldr_dir();
    let staging = dir.with_extension("new");
    // An interrupted run may have left a staging directory behind.
    let _ = fs::remove_dir_all(&staging);
    let staged = download(tag, &staging).and_then(|()| swap_dir(&staging, &dir));
    if staged.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    staged?;
    fs::write(lock_path(), format!("{tag}\n"))?;
    println!("pinned cldr-json {tag} in xtask/cldr.lock");
    Ok(())
}

fn download(tag: &str, dir: &Path) -> gen::Result<()> {
    let mut files: Vec<String> = gen::LANGUAGES
        .iter()
        .flat_map(|l| gen::lang_files(l))
        .collect();
    files.push(gen::SUPPLEMENTAL.to_string());
    for rel in &files {
        fetch(&format!("{RAW}/{tag}/cldr-json/{rel}"), &dir.join(rel))?;
        println!("fetched {rel}");
    }
    fetch(&format!("{RAW}/{tag}/LICENSE"), &dir.join("LICENSE"))?;
    println!("fetched LICENSE");
    Ok(())
}

/// Puts `new` in place of `live`. A non-empty directory cannot be replaced by
/// `rename` in one step, so the old one is moved aside and put back if the
/// second rename fails. Failing to delete the old copy afterwards is only a
/// warning.
fn swap_dir(new: &Path, live: &Path) -> gen::Result<()> {
    let old = live.with_extension("old");
    let _ = fs::remove_dir_all(&old);
    let had_live = live.exists();
    if had_live {
        fs::rename(live, &old)?;
    }
    if let Err(e) = fs::rename(new, live) {
        if had_live {
            let _ = fs::rename(&old, live);
        }
        return Err(e.into());
    }
    if had_live {
        // The new data is already live; a leftover only costs disk space and
        // is removed by the next update.
        if let Err(e) = fs::remove_dir_all(&old) {
            eprintln!("warning: could not remove {}: {e}", old.display());
        }
    }
    Ok(())
}

fn fetch(url: &str, dest: &Path) -> gen::Result<()> {
    fs::create_dir_all(dest.parent().unwrap())?;
    let status = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--output",
        ])
        .arg(dest)
        .arg(url)
        .status()
        .map_err(|e| format!("cannot run curl: {e}"))?;
    if !status.success() {
        return Err(format!("curl failed ({status}) for {url}").into());
    }
    Ok(())
}

fn read_lock() -> gen::Result<String> {
    let tag = fs::read_to_string(lock_path())
        .map_err(|e| format!("xtask/cldr.lock: {e}; run `cargo xtask cldr-update <tag>` first"))?;
    Ok(tag.trim().to_string())
}

fn generate() -> gen::Result<()> {
    let tag = read_lock()?;
    for (name, source) in gen::generate(&cldr_dir(), &tag)? {
        fs::write(data_dir().join(&name), source)?;
        println!("wrote verbalize/src/data/{name}");
    }
    Ok(())
}

/// Regenerates in memory and compares with the checked-in files.
fn check() -> gen::Result<()> {
    let tag = read_lock()?;
    let mut stale = Vec::new();
    for (name, source) in gen::generate(&cldr_dir(), &tag)? {
        match fs::read_to_string(data_dir().join(&name)) {
            Ok(current) if current == source => {}
            _ => stale.push(name),
        }
    }
    if stale.is_empty() {
        println!("verbalize/src/data is up to date with cldr-json {tag}");
        Ok(())
    } else {
        Err(format!(
            "stale generated data: {}; run `cargo xtask gen`",
            stale.join(", ")
        )
        .into())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// The data is already swapped in when the old copy cannot be removed, so
    /// that is a warning: failing here would leave `cldr.lock` naming the old
    /// tag over new data.
    #[test]
    fn swap_dir_succeeds_when_the_old_copy_cannot_be_removed() {
        let tmp = std::env::temp_dir().join(format!("xtask_swap_dir_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let (new, live, locked) = (tmp.join("new"), tmp.join("live"), tmp.join("live/locked"));
        fs::create_dir_all(&new).unwrap();
        fs::write(new.join("b.txt"), "new").unwrap();
        fs::create_dir_all(&locked).unwrap();
        fs::write(locked.join("a.txt"), "old").unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();

        // Root ignores the permission bits, so the removal cannot be made to fail.
        if fs::write(locked.join("probe"), "").is_ok() {
            let _ = fs::remove_dir_all(&tmp);
            eprintln!("skipped: permission bits are not enforced for this user");
            return;
        }
        let swapped = swap_dir(&new, &live);

        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).ok();
        let old = live.with_extension("old");
        fs::set_permissions(old.join("locked"), fs::Permissions::from_mode(0o755)).ok();
        let new_data = fs::read_to_string(live.join("b.txt")).ok();
        let _ = fs::remove_dir_all(&tmp);

        swapped.unwrap();
        assert_eq!(new_data.as_deref(), Some("new"));
    }
}
