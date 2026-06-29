//! A read model for the **spec ⟷ code** traceability/sync view.
//!
//! Harness is spec-as-source: the spec (requirements + design + tasks) is the
//! truth, and code is derived from it by the loop. This module assembles, for a
//! single spec, everything a left-to-right "Requirements → Design → Tasks →
//! Code" view needs:
//!
//! * the three spec artifacts and the traceability links between them
//!   (`task.requirements` → `requirement.id`),
//! * **generation progress** — how far the loop has turned tasks into validated
//!   code (task status + phases), and
//! * **baseline/drift state** — whether the recorded spec↔code baseline still
//!   holds, from the manifest ([`crate::manifest::check_spec`]).
//!
//! Like [`crate::snapshot`], this only reads disk; it never mutates run state.

use std::path::Path;

use anyhow::Result;
use globset::Glob;

use crate::manifest::{check_spec, expand_owned_paths, load_manifest, DriftKind};
use crate::spec::{load_requirements, load_tasks, spec_dir, Requirement, Task, TaskStatus};

/// Per-file sync state on the code side of the boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDrift {
    /// Matches the recorded baseline.
    Clean,
    /// Content changed since the baseline was recorded.
    Drifted,
    /// Recorded in the baseline but no longer on disk.
    Missing,
    /// Owned by the spec's globs but there is no baseline yet.
    Unrecorded,
}

/// One owned code file and its drift state.
#[derive(Debug, Clone)]
pub struct OwnedFile {
    pub path: String,
    pub drift: FileDrift,
}

/// Headline sync state of the whole spec↔code boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    /// No manifest baseline recorded for this spec.
    Unrecorded,
    /// Spec and code match the recorded baseline.
    Clean,
    /// Spec inputs changed since the baseline (code is behind the spec).
    Stale,
    /// One or more owned files changed or vanished out-of-band.
    Drifted,
}

impl SyncState {
    pub fn label(&self) -> &'static str {
        match self {
            SyncState::Unrecorded => "unrecorded",
            SyncState::Clean => "in sync",
            SyncState::Stale => "stale",
            SyncState::Drifted => "drift",
        }
    }
}

/// Detailed sync breakdown for a spec.
#[derive(Debug, Clone, Default)]
pub struct SpecSync {
    pub recorded: bool,
    /// Spec inputs (req+design+tasks) edited since the baseline.
    pub stale_inputs: bool,
    pub drifted_files: Vec<String>,
    pub missing_files: Vec<String>,
}

impl SpecSync {
    pub fn state(&self) -> SyncState {
        if !self.recorded {
            SyncState::Unrecorded
        } else if !self.drifted_files.is_empty() || !self.missing_files.is_empty() {
            SyncState::Drifted
        } else if self.stale_inputs {
            SyncState::Stale
        } else {
            SyncState::Clean
        }
    }

    pub fn is_clean(&self) -> bool {
        self.state() == SyncState::Clean
    }
}

/// Validated-generation rollup: how far the loop has built this spec's tasks.
#[derive(Debug, Clone, Default)]
pub struct GenProgress {
    pub todo: usize,
    pub in_progress: usize,
    pub blocked: usize,
    pub done: usize,
}

impl GenProgress {
    pub fn total(&self) -> usize {
        self.todo + self.in_progress + self.blocked + self.done
    }

    /// Fraction of tasks completed (validated) in `0.0..=1.0`.
    pub fn ratio(&self) -> f32 {
        let t = self.total();
        if t == 0 {
            0.0
        } else {
            self.done as f32 / t as f32
        }
    }
}

/// Everything the trace view renders for one spec.
pub struct SpecTrace {
    pub name: String,
    pub requirements: Vec<Requirement>,
    /// The spec's `owns` globs, kept for display/diagnostics.
    pub owns: Vec<String>,
    pub design: String,
    pub tasks: Vec<Task>,
    pub owned_files: Vec<OwnedFile>,
    pub sync: SpecSync,
    pub gen: GenProgress,
}

impl SpecTrace {
    pub fn load(root: &Path, spec_name: &str) -> Result<SpecTrace> {
        let dir = spec_dir(root, spec_name);
        let reqs = load_requirements(&dir).ok();
        let (requirements, owns) = match reqs {
            Some(rf) => (rf.requirements, rf.owns),
            None => (Vec::new(), Vec::new()),
        };
        let design = std::fs::read_to_string(dir.join("2-design.md")).unwrap_or_default();
        let tasks = load_tasks(&dir).unwrap_or_default();

        // Generation rollup.
        let mut gen = GenProgress::default();
        for t in &tasks {
            match t.status {
                TaskStatus::Todo => gen.todo += 1,
                TaskStatus::InProgress => gen.in_progress += 1,
                TaskStatus::Blocked => gen.blocked += 1,
                TaskStatus::Done => gen.done += 1,
            }
        }

        // Sync / drift from the manifest.
        let manifest = load_manifest(root)?;
        let recorded = manifest.specs.contains_key(spec_name);
        let check = check_spec(root, spec_name)?;
        let mut sync = SpecSync {
            recorded,
            ..Default::default()
        };
        for d in &check.drifts {
            match d {
                DriftKind::StaleCode { .. } => sync.stale_inputs = true,
                DriftKind::CodeDrift { path } => sync.drifted_files.push(path.clone()),
                DriftKind::Missing { path } => sync.missing_files.push(path.clone()),
                DriftKind::Unrecorded { .. } => sync.recorded = false,
            }
        }

        // Owned-file list with per-file drift.
        let mut owned_files = Vec::new();
        if let Some(entry) = manifest.specs.get(spec_name) {
            let mut paths: Vec<String> = entry.owned_files.keys().cloned().collect();
            paths.sort();
            for p in paths {
                let drift = if sync.missing_files.contains(&p) {
                    FileDrift::Missing
                } else if sync.drifted_files.contains(&p) {
                    FileDrift::Drifted
                } else {
                    FileDrift::Clean
                };
                owned_files.push(OwnedFile { path: p, drift });
            }
        } else {
            for p in expand_owned_paths(root, &owns).unwrap_or_default() {
                owned_files.push(OwnedFile {
                    path: p,
                    drift: FileDrift::Unrecorded,
                });
            }
        }

        Ok(SpecTrace {
            name: spec_name.to_string(),
            requirements,
            owns,
            design,
            tasks,
            owned_files,
            sync,
            gen,
        })
    }

    /// Tasks that declare they cover `req_id`.
    pub fn tasks_for_requirement(&self, req_id: &str) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|t| t.requirements.iter().any(|r| r == req_id))
            .collect()
    }

    /// Requirements covered by `task`.
    pub fn requirements_for_task<'a>(&'a self, task: &Task) -> Vec<&'a Requirement> {
        self.requirements
            .iter()
            .filter(|r| task.requirements.iter().any(|id| id == &r.id))
            .collect()
    }

    /// Whether a task's `files_hint` points at `path` (advisory link — the only
    /// per-task → file signal the spec carries).
    pub fn task_touches(task: &Task, path: &str) -> bool {
        task.files_hint.iter().any(|hint| hint_matches(hint, path))
    }
}

fn hint_matches(hint: &str, path: &str) -> bool {
    if hint == path || path.starts_with(hint) {
        return true;
    }
    Glob::new(hint)
        .map(|g| g.compile_matcher().is_match(path))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::record_spec;
    use std::path::PathBuf;

    fn fixture(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("harness-trace-{}-{tag}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        let spec = dir.join(".specs").join("demo");
        std::fs::create_dir_all(&spec).unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();

        std::fs::write(
            spec.join("1-requirements.json"),
            r#"{"spec":"demo","version":"1","requirements":[
                {"id":"R-1","text":"parse empty"},
                {"id":"R-2","text":"round-trip"}],
                "owns":["src/**"]}"#,
        )
        .unwrap();
        std::fs::write(
            spec.join("2-design.md"),
            "R-1 in parser; R-2 in serializer\n",
        )
        .unwrap();
        let t1 = r#"{"id":"T-001","spec":"demo","title":"parser","status":"done","priority":1,"requirements":["R-1"],"files_hint":["src/parser.rs"],"created_at":"2026-06-22T11:00:00Z","updated_at":"2026-06-22T12:00:00Z"}"#;
        let t2 = r#"{"id":"T-002","spec":"demo","title":"ser","status":"todo","priority":2,"requirements":["R-2"],"files_hint":["src/serializer.rs"],"created_at":"2026-06-22T11:00:00Z","updated_at":"2026-06-22T12:00:00Z"}"#;
        std::fs::write(spec.join("3-tasks.jsonl"), format!("{t1}\n{t2}\n")).unwrap();
        std::fs::write(dir.join("src").join("parser.rs"), "fn parse() {}\n").unwrap();
        std::fs::write(dir.join("src").join("serializer.rs"), "fn ser() {}\n").unwrap();
        dir
    }

    #[test]
    fn unrecorded_then_clean_then_drift() {
        let root = fixture("sync");

        // No manifest yet → unrecorded, owned files discovered via globs.
        let tr = SpecTrace::load(&root, "demo").unwrap();
        assert_eq!(tr.sync.state(), SyncState::Unrecorded);
        assert_eq!(tr.owned_files.len(), 2);
        assert!(tr
            .owned_files
            .iter()
            .all(|f| f.drift == FileDrift::Unrecorded));
        assert_eq!(tr.gen.done, 1);
        assert_eq!(tr.gen.total(), 2);

        // Record a baseline → clean.
        record_spec(&root, "demo").unwrap();
        let tr = SpecTrace::load(&root, "demo").unwrap();
        assert_eq!(tr.sync.state(), SyncState::Clean);
        assert!(tr.owned_files.iter().all(|f| f.drift == FileDrift::Clean));

        // Hand-edit an owned file → drift on that file only.
        std::fs::write(
            root.join("src").join("serializer.rs"),
            "fn ser() { /*x*/ }\n",
        )
        .unwrap();
        let tr = SpecTrace::load(&root, "demo").unwrap();
        assert_eq!(tr.sync.state(), SyncState::Drifted);
        assert_eq!(tr.sync.drifted_files, vec!["src/serializer.rs".to_string()]);
        let parser = tr
            .owned_files
            .iter()
            .find(|f| f.path == "src/parser.rs")
            .unwrap();
        assert_eq!(parser.drift, FileDrift::Clean);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn traceability_links() {
        let root = fixture("trace");
        let tr = SpecTrace::load(&root, "demo").unwrap();

        let r1_tasks = tr.tasks_for_requirement("R-1");
        assert_eq!(r1_tasks.len(), 1);
        assert_eq!(r1_tasks[0].id, "T-001");

        let t1 = tr.tasks.iter().find(|t| t.id == "T-001").unwrap();
        let reqs = tr.requirements_for_task(t1);
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].id, "R-1");
        assert!(SpecTrace::task_touches(t1, "src/parser.rs"));
        assert!(!SpecTrace::task_touches(t1, "src/serializer.rs"));

        std::fs::remove_dir_all(&root).ok();
    }
}
