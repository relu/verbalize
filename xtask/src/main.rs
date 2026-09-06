//! `cargo xtask cldr-update <tag> | gen | check-data`
//!
//! Vendors the CLDR JSON the library needs (design spec §6.1) and compiles it
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

/// Downloads the §6.1 files for every supported language plus the upstream
/// LICENSE into `xtask/cldr/`, replacing whatever was there, and records
/// the tag in `xtask/cldr.lock`.
fn update(tag: &str) -> gen::Result<()> {
    let dir = cldr_dir();
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
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
    fs::write(lock_path(), format!("{tag}\n"))?;
    println!("pinned cldr-json {tag} in xtask/cldr.lock");
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
