use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::model::{Commit, FileStat, GitRef, RefKind, Repo};

#[derive(Debug, Clone)]
pub struct LoadOptions {
    pub all: bool,
    pub limit: usize,
    pub since: Option<String>,
    pub author: Option<String>,
    pub no_merges: bool,
}

impl Default for LoadOptions {
    fn default() -> Self {
        LoadOptions {
            all: false,
            limit: 5000,
            since: None,
            author: None,
            no_merges: false,
        }
    }
}

const PRETTY: &str = "--pretty=format:%x1e%H%x1f%h%x1f%an%x1f%ae%x1f%at%x1f%s%x1f%D%x1f%P";

fn run(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .args(args)
        .output()
        .with_context(|| "failed to run `git` (is it installed?)")?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.first().unwrap_or(&""), err.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Full commit record: header, message, stat and patch.
pub fn show(root: &Path, hash: &str) -> Result<Vec<String>> {
    let raw = run(
        root,
        &[
            "show",
            "--no-color",
            "--stat",
            "--patch",
            "--format=fuller",
            hash,
        ],
    )?;
    let mut lines: Vec<String> = raw.lines().map(|l| l.trim_end().to_string()).collect();
    const MAX_LINES: usize = 8_000;
    if lines.len() > MAX_LINES {
        lines.truncate(MAX_LINES);
        lines.push("… diff truncated by gitquake …".to_string());
    }
    Ok(lines)
}

pub fn load(path: &Path, opts: &LoadOptions) -> Result<Repo> {
    if !path.exists() {
        bail!("path does not exist: {}", path.display());
    }

    let root = run(path, &["rev-parse", "--show-toplevel"])
        .map(|s| PathBuf::from(s.trim().to_string()))
        .map_err(|err| {
            let msg = err.to_string();
            if msg.contains("not a git repository") {
                anyhow::anyhow!("{} is not inside a git repository", path.display())
            } else {
                err
            }
        })?;

    let name = root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| root.display().to_string());

    let branch = match run(&root, &["symbolic-ref", "--short", "-q", "HEAD"]) {
        Ok(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => match run(&root, &["rev-parse", "--short", "HEAD"]) {
            Ok(s) if !s.trim().is_empty() => format!("detached @ {}", s.trim()),
            _ => "empty".into(),
        },
    };

    let count_arg = if opts.all { "--all" } else { "HEAD" };
    let total_commits = run(&root, &["rev-list", "--count", count_arg])
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(0);

    let limit_arg;
    let mut args: Vec<&str> = vec![
        "-c",
        "core.quotepath=false",
        "log",
        "--date-order",
        "--numstat",
        PRETTY,
    ];
    if opts.all {
        args.push("--all");
    }
    if opts.no_merges {
        args.push("--no-merges");
    }
    if let Some(since) = &opts.since {
        args.push("--since");
        args.push(since);
    }
    if let Some(author) = &opts.author {
        args.push("--author");
        args.push(author);
    }
    if opts.limit > 0 {
        limit_arg = opts.limit.to_string();
        args.push("-n");
        args.push(&limit_arg);
    }

    let raw = match run(&root, &args) {
        Ok(s) => s,
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("does not have any commits yet")
                || msg.contains("your current branch")
                || msg.contains("unknown revision")
            {
                String::new()
            } else {
                return Err(err);
            }
        }
    };

    let mut commits = parse_log(&raw);
    commits.sort_by_key(|c| std::cmp::Reverse(c.timestamp));

    let authors = commits
        .iter()
        .map(|c| c.author_email.to_ascii_lowercase())
        .collect::<HashSet<_>>()
        .len();

    Ok(Repo {
        root,
        name,
        branch,
        total_commits,
        authors,
        commits,
    })
}

pub fn parse_log(out: &str) -> Vec<Commit> {
    out.split('\x1e')
        .filter(|record| !record.trim().is_empty())
        .filter_map(parse_record)
        .collect()
}

fn parse_record(record: &str) -> Option<Commit> {
    let mut fields = record.splitn(8, '\x1f');
    let hash = fields.next()?.trim().to_string();
    if hash.len() < 4 {
        return None;
    }
    let short = fields.next().unwrap_or("").trim().to_string();
    let author_name = fields.next().unwrap_or("").trim().to_string();
    let author_email = fields.next().unwrap_or("").trim().to_string();
    let timestamp = fields
        .next()
        .unwrap_or("0")
        .trim()
        .parse::<i64>()
        .unwrap_or(0);
    let subject = fields.next().unwrap_or("").trim().to_string();
    let refs_raw = fields.next().unwrap_or("");
    let tail = fields.next().unwrap_or("");

    let mut lines = tail.lines();
    let parents: Vec<String> = lines
        .next()
        .unwrap_or("")
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let mut files = Vec::new();
    let mut insertions = 0u64;
    let mut deletions = 0u64;
    for line in lines {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, '\t');
        let add = parts.next().unwrap_or("");
        let del = parts.next().unwrap_or("");
        let path = parts.next().unwrap_or("").to_string();
        if path.is_empty() {
            continue;
        }
        let ins = add.trim().parse::<u64>().ok();
        let dels = del.trim().parse::<u64>().ok();
        insertions += ins.unwrap_or(0);
        deletions += dels.unwrap_or(0);
        files.push(FileStat {
            path,
            insertions: ins,
            deletions: dels,
        });
    }

    Some(Commit {
        hash: hash.trim().to_string(),
        short: if short.is_empty() {
            hash.chars().take(7).collect()
        } else {
            short
        },
        author_name,
        author_email,
        timestamp,
        subject,
        refs: parse_refs(refs_raw),
        parents,
        files,
        insertions,
        deletions,
    })
}

pub fn parse_refs(raw: &str) -> Vec<GitRef> {
    let mut out = Vec::new();
    for part in raw.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(name) = part.strip_prefix("HEAD -> ") {
            out.push(GitRef {
                kind: RefKind::Head,
                name: name.trim().to_string(),
            });
            continue;
        }
        if let Some(name) = part.strip_prefix("tag: ") {
            out.push(GitRef {
                kind: RefKind::Tag,
                name: name.trim().to_string(),
            });
            continue;
        }
        if part == "HEAD" {
            out.push(GitRef {
                kind: RefKind::Head,
                name: "HEAD".into(),
            });
            continue;
        }
        let kind = if part.starts_with("refs/heads/") || part.starts_with("heads/") {
            RefKind::Branch
        } else if part.contains('/') {
            RefKind::Remote
        } else {
            RefKind::Branch
        };
        let name = part.strip_prefix("refs/heads/").unwrap_or(part).to_string();
        out.push(GitRef { kind, name });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const REC: &str = "\x1ea1b2c3d4e5f60718293a4b5c6d7e8f9012345678\x1fa1b2c3d\x1fAda Lovelace\x1fada@example.com\x1f1700000000\x1ffeat: add parser\x1fHEAD -> main, tag: v0.1, origin/main\x1fdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef\n12\t3\tsrc/parser.rs\n-\t-\tassets/logo.png\n2\t0\t文档/说明.md\n\n";

    #[test]
    fn parses_a_full_record() {
        let commits = parse_log(REC);
        assert_eq!(commits.len(), 1);
        let c = &commits[0];
        assert_eq!(c.hash.len(), 40);
        assert_eq!(c.short, "a1b2c3d");
        assert_eq!(c.author_name, "Ada Lovelace");
        assert_eq!(c.author_email, "ada@example.com");
        assert_eq!(c.timestamp, 1_700_000_000);
        assert_eq!(c.subject, "feat: add parser");
        assert_eq!(c.parents, vec!["deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"]);
        assert_eq!(c.files.len(), 3);
        assert_eq!(c.insertions, 14);
        assert_eq!(c.deletions, 3);
        assert_eq!(c.volume(), 17);
        assert!(c.files[1].is_binary());
        assert_eq!(c.files[2].path, "文档/说明.md");
        assert_eq!(c.refs.len(), 3);
        assert_eq!(c.refs[0].kind, RefKind::Head);
        assert_eq!(c.refs[0].name, "main");
        assert_eq!(c.refs[1].kind, RefKind::Tag);
        assert_eq!(c.refs[2].kind, RefKind::Remote);
    }

    #[test]
    fn parses_merge_without_numstat() {
        let rec = "\x1eabc12345\x1fabc1234\x1fBob\x1fbob@x\x1f1700000001\x1fMerge branch 'x'\x1f\x1f1111111111111111111111111111111111111111 2222222222222222222222222222222222222222\n\n";
        let commits = parse_log(rec);
        assert_eq!(commits.len(), 1);
        assert!(commits[0].is_merge());
        assert!(commits[0].files.is_empty());
        assert_eq!(commits[0].diffstat(), "merge");
        assert!(commits[0].refs.is_empty());
    }

    #[test]
    fn parses_renames_and_multiple_records() {
        let out = format!(
            "{}\x1ef00d1234\x1ff00d123\x1fEve\x1feve@x\x1f1700000002\x1frename\x1f\x1f{}\n3\t4\tsrc/{{old => new}}/mod.rs\n\n",
            REC,
            "a".repeat(40)
        );
        let commits = parse_log(&out);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[1].files[0].path, "src/{old => new}/mod.rs");
        assert_eq!(commits[1].insertions, 3);
        assert_eq!(commits[1].deletions, 4);
    }

    #[test]
    fn ignores_garbage_records() {
        assert!(parse_log("").is_empty());
        assert!(parse_log("\x1e\n\n").is_empty());
    }

    #[test]
    fn ref_kinds() {
        let refs = parse_refs("HEAD -> main, origin/feature, v1.0, refs/heads/dev");
        assert_eq!(refs[0].kind, RefKind::Head);
        assert_eq!(refs[1].kind, RefKind::Remote);
        assert_eq!(refs[2].kind, RefKind::Branch);
        assert_eq!(refs[3].kind, RefKind::Branch);
        assert!(parse_refs("").is_empty());
    }
}
