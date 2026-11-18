use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ghg")
}

fn run(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("failed to run ghg")
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to run git");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn temp_repo(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ghg-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "Test User"]);
    dir
}

fn commit(dir: &Path, file: &str, body: &str, msg: &str) {
    fs::write(dir.join(file), body).unwrap();
    git(dir, &["add", file]);
    git(dir, &["commit", "-q", "-m", msg]);
}

#[test]
fn plain_rendering_shows_history() {
    let dir = temp_repo("plain");
    commit(&dir, "small.txt", "one\n", "add a tiny file");
    commit(
        &dir,
        "big.txt",
        &(0..80).map(|i| format!("line {i}\n")).collect::<String>(),
        "add a large file",
    );
    commit(&dir, "small.txt", "one\ntwo\n", "tweak the tiny file");

    let out = run(&[
        "--no-config",
        "--plain",
        "--no-color",
        dir.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("commits"), "{text}");
    assert!(text.contains("add a large file"), "{text}");
    assert!(text.contains("add a tiny file"), "{text}");
    assert!(text.contains("heat"), "{text}");
    // waveform uses eighth blocks or full blocks
    assert!(
        text.chars().any(|c| "▁▂▃▄▅▆▇█".contains(c)),
        "expected waveform glyphs:\n{text}"
    );
    assert!(text.contains('+'), "{text}");
}

#[test]
fn dump_config_mentions_key_settings() {
    let out = run(&["--no-config", "--dump-config"]);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("[general]"), "{text}");
    assert!(text.contains("theme = \"inferno\""), "{text}");
    assert!(text.contains("wave_axis = \"blend\""), "{text}");
    assert!(text.contains("heat_percentile = 95"), "{text}");
    assert!(text.contains("row_fields = "), "{text}");
}

#[test]
fn list_themes_prints_builtins() {
    let out = run(&["--no-config", "--list-themes"]);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    for name in ["inferno", "magma", "viridis", "ice", "mono", "solarized"] {
        assert!(text.contains(name), "missing {name}:\n{text}");
    }
}

#[test]
fn outside_a_repository_fails_gracefully() {
    let dir = std::env::temp_dir().join(format!("ghg-test-norepo-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let out = run(&["--no-config", "--plain", dir.to_str().unwrap()]);
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("git repository"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn all_branches_flag_includes_other_branches() {
    let dir = temp_repo("branches");
    commit(&dir, "a.txt", "a\n", "commit on main");
    git(&dir, &["checkout", "-q", "-b", "side"]);
    commit(&dir, "b.txt", "b\n", "side-only change");
    git(&dir, &["checkout", "-q", "main"]);

    let current = run(&[
        "--no-config",
        "--plain",
        "--no-color",
        dir.to_str().unwrap(),
    ]);
    let current = String::from_utf8_lossy(&current.stdout);
    assert!(!current.contains("side-only change"), "{current}");

    let all = run(&[
        "--no-config",
        "--plain",
        "--no-color",
        "--all",
        dir.to_str().unwrap(),
    ]);
    let all = String::from_utf8_lossy(&all.stdout);
    assert!(all.contains("side-only change"), "{all}");
}
