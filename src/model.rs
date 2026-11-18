use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    Head,
    Branch,
    Tag,
    Remote,
    #[allow(dead_code)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRef {
    pub kind: RefKind,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStat {
    pub path: String,
    /// `None` means the file is binary (git prints `-`).
    pub insertions: Option<u64>,
    pub deletions: Option<u64>,
}

impl FileStat {
    pub fn volume(&self) -> u64 {
        self.insertions.unwrap_or(0) + self.deletions.unwrap_or(0)
    }

    pub fn is_binary(&self) -> bool {
        self.insertions.is_none()
    }
}

#[derive(Debug, Clone)]
pub struct Commit {
    pub hash: String,
    pub short: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: i64,
    pub subject: String,
    pub refs: Vec<GitRef>,
    pub parents: Vec<String>,
    pub files: Vec<FileStat>,
    pub insertions: u64,
    pub deletions: u64,
}

impl Commit {
    pub fn volume(&self) -> u64 {
        self.insertions + self.deletions
    }

    pub fn is_merge(&self) -> bool {
        self.parents.len() > 1
    }

    /// Change volume used for heat: merges are shown cold because git does not
    /// report a diffstat for them by default.
    pub fn heat_volume(&self) -> u64 {
        if self.is_merge() { 0 } else { self.volume() }
    }

    #[allow(dead_code)]
    pub fn diffstat(&self) -> String {
        if self.is_merge() {
            return "merge".into();
        }
        if self.files.iter().any(|f| f.is_binary()) && self.volume() == 0 {
            return "binary".into();
        }
        format!("+{} -{}", self.insertions, self.deletions)
    }
}

#[derive(Debug, Clone)]
pub struct Repo {
    pub root: PathBuf,
    pub name: String,
    pub branch: String,
    pub total_commits: usize,
    pub authors: usize,
    pub commits: Vec<Commit>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(parents: &[&str], ins: Option<u64>, del: Option<u64>) -> Commit {
        Commit {
            hash: "a".repeat(40),
            short: "aaaaaaa".into(),
            author_name: "T".into(),
            author_email: "t@t".into(),
            timestamp: 0,
            subject: "s".into(),
            refs: vec![],
            parents: parents.iter().map(|s| s.to_string()).collect(),
            files: vec![FileStat {
                path: "f".into(),
                insertions: ins,
                deletions: del,
            }],
            insertions: ins.unwrap_or(0),
            deletions: del.unwrap_or(0),
        }
    }

    #[test]
    fn merge_has_no_heat_and_merge_label() {
        let c = commit(&["a", "b"], None, None);
        assert!(c.is_merge());
        assert_eq!(c.heat_volume(), 0);
        assert_eq!(c.diffstat(), "merge");
    }

    #[test]
    fn binary_file_detected() {
        let c = commit(&["a"], None, None);
        assert!(c.files[0].is_binary());
        assert_eq!(c.diffstat(), "binary");
    }

    #[test]
    fn volume_adds_both_sides() {
        let c = commit(&["a"], Some(10), Some(5));
        assert_eq!(c.volume(), 15);
        assert_eq!(c.diffstat(), "+10 -5");
    }
}
