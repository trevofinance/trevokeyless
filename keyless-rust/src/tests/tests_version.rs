//! Pins the version declarations this crate can see to `CARGO_PKG_VERSION` — the
//! per-package counterpart of the Python `tests/test_version.py`. The repository-wide
//! release helper (`.github/scripts/release_metadata.py`, run by check-docs) verifies
//! the full set of sync points; these tests make a Rust-only drift fail the crate's own
//! suite instead of only the docs gate. (`Cargo.lock` needs no test: every CI cargo
//! invocation passes `--locked`, which is the pin.)

/// The README ships inside the published crate (`readme = "README.md"` in Cargo.toml),
/// so a stale install snippet is published verbatim to crates.io and docs.rs.
#[test]
fn test_readme_install_snippet_matches_the_crate_version() {
    let readme = include_str!("../../README.md");
    let snippet = readme
        .lines()
        .find(|line| line.starts_with("trevokeyless = { version = \""))
        .expect("the README must contain the dependency install snippet");
    let version = snippet
        .split('"')
        .nth(1)
        .expect("the snippet pins a quoted version");
    assert_eq!(
        version,
        env!("CARGO_PKG_VERSION"),
        "the README install snippet must match Cargo.toml \
         (release_metadata.py --set rust rewrites both)"
    );
}

/// The fuzz harness is a standalone workspace whose lockfile also pins this crate (a
/// path dependency) and is audited separately (`cargo audit --file fuzz/Cargo.lock`).
/// Repository-only: `fuzz/` is excluded from the published crate, so — like the
/// conformance fixture test — this skips when the directory is absent.
#[test]
fn test_fuzz_lockfile_pins_the_crate_version() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fuzz/Cargo.lock");
    let Ok(lock) = std::fs::read_to_string(path) else {
        eprintln!("fuzz lockfile not found; skipping (running from a published crate?)");
        return;
    };
    // Line-based, not a `contains` of a `\n`-joined pair: a Windows checkout with
    // `core.autocrlf=true` has CRLF line endings, and `lines()` normalizes them away.
    let version = format!("version = \"{}\"", env!("CARGO_PKG_VERSION"));
    let pinned = lock
        .lines()
        .zip(lock.lines().skip(1))
        .any(|(name_line, next_line)| {
            name_line == "name = \"trevokeyless\"" && next_line == version
        });
    assert!(
        pinned,
        "fuzz/Cargo.lock must pin trevokeyless at {} \
         (release_metadata.py --set rust rewrites it)",
        env!("CARGO_PKG_VERSION")
    );
}
