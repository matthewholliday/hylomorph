//! The opinionated five-layer model for a spec's vertical slice.
//!
//! Every spec is produced in a strict order:
//!
//! ```text
//! requirements → design → tasks → code → evals
//! ```
//!
//! A downstream layer may only be produced once *every* upstream layer already
//! exists. These gates are enforced here — in the harness, before any agent is
//! ever launched — so it is structurally impossible to draft a design without
//! requirements, generate code without tasks, and so on. The ordering is never
//! left to the agent's judgement.
//!
//! Each layer corresponds to a concrete artifact on disk:
//!
//! | layer        | artifact                              |
//! |--------------|---------------------------------------|
//! | requirements | `.specs/<spec>/1-requirements.json`   |
//! | design       | `.specs/<spec>/2-design.md`           |
//! | tasks        | `.specs/<spec>/3-tasks.jsonl`         |
//! | code         | files matched by the spec's `owns` globs |
//! | evals        | `evals/<spec>/*`                      |

use std::path::Path;

use crate::manifest::expand_owned_paths;
use crate::spec::{load_requirements, load_tasks, spec_dir};

/// One layer of a spec's vertical slice, in production order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Requirements,
    Design,
    Tasks,
    Code,
    Evals,
}

impl Layer {
    /// Every layer, in production order.
    pub const ALL: [Layer; 5] = [
        Layer::Requirements,
        Layer::Design,
        Layer::Tasks,
        Layer::Code,
        Layer::Evals,
    ];

    /// Human-readable name used in messages.
    pub fn label(self) -> &'static str {
        match self {
            Layer::Requirements => "requirements",
            Layer::Design => "design",
            Layer::Tasks => "tasks",
            Layer::Code => "code",
            Layer::Evals => "evals",
        }
    }

    /// The exact command a user runs to produce this layer.
    pub fn produce_cmd(self, spec: &str) -> String {
        match self {
            Layer::Requirements => format!("harness spec requirements {spec} --brief \"…\""),
            Layer::Design => format!("harness spec design {spec}"),
            Layer::Tasks => format!("harness spec tasks {spec}"),
            Layer::Code => format!("harness build {spec}"),
            Layer::Evals => format!("harness eval draft {spec}"),
        }
    }

    /// The upstream layers that must exist before this one can be produced.
    pub fn upstream(self) -> &'static [Layer] {
        match self {
            Layer::Requirements => &[],
            Layer::Design => &[Layer::Requirements],
            Layer::Tasks => &[Layer::Requirements, Layer::Design],
            Layer::Code => &[Layer::Requirements, Layer::Design, Layer::Tasks],
            Layer::Evals => &[
                Layer::Requirements,
                Layer::Design,
                Layer::Tasks,
                Layer::Code,
            ],
        }
    }
}

/// Whether a layer's artifact exists and is usable as an upstream input.
#[derive(Debug, Clone)]
pub enum LayerStatus {
    /// The artifact does not exist yet.
    Absent,
    /// The artifact exists and parses / is non-empty.
    Present,
    /// The artifact exists but is malformed; carries a short reason.
    Invalid(String),
}

impl LayerStatus {
    pub fn is_present(&self) -> bool {
        matches!(self, LayerStatus::Present)
    }

    /// A short glyph + word for status displays.
    pub fn glyph(&self) -> &'static str {
        match self {
            LayerStatus::Present => "✓",
            LayerStatus::Absent => "·",
            LayerStatus::Invalid(_) => "✗",
        }
    }
}

/// The status of all five layers for one spec.
pub struct LayerState {
    pub requirements: LayerStatus,
    pub design: LayerStatus,
    pub tasks: LayerStatus,
    pub code: LayerStatus,
    pub evals: LayerStatus,
}

impl LayerState {
    pub fn status(&self, layer: Layer) -> &LayerStatus {
        match layer {
            Layer::Requirements => &self.requirements,
            Layer::Design => &self.design,
            Layer::Tasks => &self.tasks,
            Layer::Code => &self.code,
            Layer::Evals => &self.evals,
        }
    }

    /// The next layer that can be produced, or `None` if all five exist.
    pub fn next_producible(&self) -> Option<Layer> {
        Layer::ALL
            .iter()
            .copied()
            .find(|&l| !self.status(l).is_present())
    }
}

/// Resolve the on-disk status of every layer for `spec`.
///
/// This performs only the lightweight checks needed to decide whether a layer
/// can serve as an upstream input (does it exist and parse?). The full
/// well-formedness gate — acceptance-criteria coverage, design headings, task
/// graph integrity — lives in `harness check`.
pub fn layer_state(root: &Path, spec: &str) -> LayerState {
    let dir = spec_dir(root, spec);

    // ── Requirements ─────────────────────────────────────────────────────────
    let requirements = {
        let path = dir.join("1-requirements.json");
        if !path.exists() {
            LayerStatus::Absent
        } else {
            match load_requirements(&dir) {
                Ok(_) => LayerStatus::Present,
                Err(e) => LayerStatus::Invalid(short_err(&e)),
            }
        }
    };

    // ── Design ───────────────────────────────────────────────────────────────
    let design = {
        let path = dir.join("2-design.md");
        match std::fs::read_to_string(&path) {
            Err(_) => LayerStatus::Absent,
            Ok(s) if s.trim().is_empty() => {
                LayerStatus::Invalid("2-design.md is empty".to_string())
            }
            Ok(_) => LayerStatus::Present,
        }
    };

    // ── Tasks ────────────────────────────────────────────────────────────────
    let tasks = {
        let path = dir.join("3-tasks.jsonl");
        if !path.exists() {
            LayerStatus::Absent
        } else {
            match load_tasks(&dir) {
                Ok(ts) if ts.is_empty() => {
                    LayerStatus::Invalid("3-tasks.jsonl has no tasks".to_string())
                }
                Ok(_) => LayerStatus::Present,
                Err(e) => LayerStatus::Invalid(short_err(&e)),
            }
        }
    };

    // ── Code ─────────────────────────────────────────────────────────────────
    // Code "exists" when the spec declares `owns` globs and at least one matched
    // file is present on disk. This is independent of the build manifest so it
    // reflects reality even if a build was never recorded.
    let code = match load_requirements(&dir) {
        Err(_) => LayerStatus::Absent,
        Ok(reqs) if reqs.owns.is_empty() => LayerStatus::Absent,
        Ok(reqs) => match expand_owned_paths(root, &reqs.owns) {
            Ok(paths) if paths.is_empty() => LayerStatus::Absent,
            Ok(_) => LayerStatus::Present,
            Err(e) => LayerStatus::Invalid(short_err(&e)),
        },
    };

    // ── Evals ────────────────────────────────────────────────────────────────
    let evals = {
        let evals_dir = root.join("evals").join(spec);
        let has_file = std::fs::read_dir(&evals_dir)
            .map(|rd| rd.filter_map(|e| e.ok()).any(|e| e.path().is_file()))
            .unwrap_or(false);
        if has_file {
            LayerStatus::Present
        } else {
            LayerStatus::Absent
        }
    };

    LayerState {
        requirements,
        design,
        tasks,
        code,
        evals,
    }
}

/// Refuse `action` unless every layer in `needed` is present.
///
/// This is the structural gate: callers invoke it *before* composing a prompt
/// or launching an agent, so an out-of-order action fails fast with the exact
/// command that would unblock it.
pub fn require(
    state: &LayerState,
    action: &str,
    needed: &[Layer],
    spec: &str,
) -> anyhow::Result<()> {
    for &layer in needed {
        match state.status(layer) {
            LayerStatus::Present => {}
            LayerStatus::Absent => anyhow::bail!(
                "cannot {action}: the '{}' layer does not exist yet.\n  \
                 produce it first:  {}",
                layer.label(),
                layer.produce_cmd(spec),
            ),
            LayerStatus::Invalid(why) => anyhow::bail!(
                "cannot {action}: the '{}' layer is present but invalid: {why}\n  \
                 fix it first:  harness check {spec}",
                layer.label(),
            ),
        }
    }
    Ok(())
}

/// Collapse an error chain to a single short line for status reporting.
fn short_err(e: &anyhow::Error) -> String {
    e.to_string()
        .lines()
        .next()
        .unwrap_or("invalid")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    /// A throwaway project root under the system temp dir.
    fn tmp_root() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("harness-layers-test-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_requirements(root: &Path, spec: &str, owns: &str) {
        let dir = spec_dir(root, spec);
        std::fs::create_dir_all(&dir).unwrap();
        let json = format!(
            r#"{{"spec":"{spec}","version":"1","owns":["{owns}"],
            "requirements":[{{"id":"REQ-001","text":"The system shall work.",
            "acceptance_criteria":["it works"]}}]}}"#
        );
        std::fs::write(dir.join("1-requirements.json"), json).unwrap();
    }

    #[test]
    fn ordering_gate_blocks_until_upstream_exists() {
        let root = tmp_root();
        let spec = "demo";

        // Nothing yet: design cannot be drafted.
        let state = layer_state(&root, spec);
        assert!(!state.requirements.is_present());
        assert!(require(&state, "draft design", &[Layer::Requirements], spec).is_err());
        assert_eq!(state.next_producible(), Some(Layer::Requirements));

        // Add requirements: now design is the next allowed action.
        write_requirements(&root, spec, "src/demo/**");
        let state = layer_state(&root, spec);
        assert!(state.requirements.is_present());
        assert!(require(&state, "draft design", &[Layer::Requirements], spec).is_ok());
        assert!(require(
            &state,
            "draft tasks",
            &[Layer::Requirements, Layer::Design],
            spec
        )
        .is_err());
        assert_eq!(state.next_producible(), Some(Layer::Design));

        // Add design: tasks unblocks, code/evals still blocked.
        std::fs::write(
            spec_dir(&root, spec).join("2-design.md"),
            "## Context\nstub\n",
        )
        .unwrap();
        let state = layer_state(&root, spec);
        assert!(require(
            &state,
            "draft tasks",
            &[Layer::Requirements, Layer::Design],
            spec
        )
        .is_ok());
        assert_eq!(state.next_producible(), Some(Layer::Tasks));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn code_layer_tracks_owned_files() {
        let root = tmp_root();
        let spec = "demo";
        write_requirements(&root, spec, "src/demo/**");

        // owns glob declared but no matching file → code absent.
        let state = layer_state(&root, spec);
        assert!(!state.code.is_present());

        // Create a matching file → code present.
        std::fs::create_dir_all(root.join("src/demo")).unwrap();
        std::fs::write(root.join("src/demo/lib.rs"), "// code\n").unwrap();
        let state = layer_state(&root, spec);
        assert!(state.code.is_present());

        let _ = std::fs::remove_dir_all(&root);
    }
}
