//! Write-scope isolation for layer-producing commands.
//!
//! A layer command may only create or modify the file(s) that layer owns. We
//! enforce this structurally rather than by asking the agent nicely: snapshot
//! the working tree's dirty set *before* the agent runs, then after it returns
//! revert anything it touched outside the allowed globs. Pre-existing local
//! edits (already dirty before the run) are never disturbed.
//!
//! This is the second half of the layer guarantee. The precondition gate in
//! [`crate::layers`] makes it impossible to run a layer command out of order;
//! this makes it impossible for a layer command to leak writes into a *different*
//! layer's files. Both are deterministic and live in the harness.
//!
//! Enforcement is git-based. With no git repository it degrades to deleting
//! out-of-scope *untracked* files only (tracked files cannot be restored without
//! history) — the ordering gate still holds regardless.

use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};

/// The set of paths a layer command is permitted to write.
pub struct WriteScope {
    allow: Option<GlobSet>,
}

impl WriteScope {
    /// Build a scope from relative glob patterns (e.g. `.specs/foo/2-design.md`).
    pub fn new(patterns: &[String]) -> Result<Self> {
        if patterns.is_empty() {
            return Ok(Self { allow: None });
        }
        let mut builder = GlobSetBuilder::new();
        for pat in patterns {
            let glob = Glob::new(pat)
                .map_err(|e| anyhow::anyhow!("invalid write-scope glob '{pat}': {e}"))?;
            builder.add(glob);
        }
        Ok(Self {
            allow: Some(builder.build()?),
        })
    }

    fn allows(&self, path: &str) -> bool {
        match &self.allow {
            Some(set) => set.is_match(path),
            None => false,
        }
    }
}

/// Paths with uncommitted changes right now: tracked-but-modified ∪ untracked.
///
/// Captured before an agent runs so enforcement only ever touches files the
/// agent itself introduced.
pub fn dirty_paths(root: &Path) -> HashSet<String> {
    let mut set = HashSet::new();
    for line in git_lines(root, &["diff", "--name-only", "HEAD"]) {
        set.insert(line);
    }
    for line in git_lines(root, &["ls-files", "--others", "--exclude-standard"]) {
        set.insert(line);
    }
    set
}

/// Revert every path the agent newly touched that falls outside `scope`.
///
/// `before` is the dirty set captured by [`dirty_paths`] prior to the run.
/// Returns the list of reverted paths (empty when the agent stayed in scope).
pub fn enforce(root: &Path, before: &HashSet<String>, scope: &WriteScope) -> Vec<String> {
    let modified: HashSet<String> = git_lines(root, &["diff", "--name-only", "HEAD"])
        .into_iter()
        .collect();
    let untracked: HashSet<String> =
        git_lines(root, &["ls-files", "--others", "--exclude-standard"])
            .into_iter()
            .collect();

    let mut reverted = Vec::new();

    for path in modified.iter().chain(untracked.iter()) {
        if before.contains(path) || scope.allows(path) {
            continue;
        }
        if untracked.contains(path) {
            // Newly created by the agent: delete it.
            if std::fs::remove_file(root.join(path)).is_ok() {
                reverted.push(path.clone());
            }
        } else {
            // Tracked file the agent modified: restore the committed version.
            let ok = Command::new("git")
                .args(["checkout", "HEAD", "--", path])
                .current_dir(root)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if ok {
                reverted.push(path.clone());
            }
        }
    }

    reverted.sort();
    reverted.dedup();
    reverted
}

fn git_lines(root: &Path, args: &[&str]) -> Vec<String> {
    let out = match Command::new("git").args(args).current_dir(root).output() {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}
