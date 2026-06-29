//! Trace mode: a two-column **Spec ⟷ Code** view.
//!
//! The manifest tracks one boundary per spec: the spec inputs *as a unit*
//! (requirements + design + tasks, combined into one hash) versus the owned code
//! (hashed per file). So the left column lists every specification, each as one
//! self-contained block — its requirements, its design, its tasks together — and
//! the right column lists each spec's owned code. A spec is exactly one of each.
//!
//! Spec-as-source means sync is directional: spec → code (forward) is driven by
//! `build`/`rebuild`; the reverse is only *detected* as drift and resolved by
//! reverting code or accepting a new baseline. Each block carries its own sync
//! state and forward-sync actions.

use eframe::egui;
use egui::{Color32, RichText};

use harness_core::trace::{FileDrift, SpecTrace, SyncState};

use crate::{HarnessApp, ACCENT, DIM, FAIL, OK, WARN};

fn sync_color(s: SyncState) -> Color32 {
    match s {
        SyncState::Clean => OK,
        SyncState::Stale => WARN,
        SyncState::Drifted => FAIL,
        SyncState::Unrecorded => DIM,
    }
}

pub fn ui(app: &mut HarnessApp, ctx: &egui::Context) {
    app.ensure_traces();

    egui::TopBottomPanel::bottom("trace-log")
        .resizable(true)
        .default_height(120.0)
        .show(ctx, |ui| {
            action_log(app, ui);
        });

    // Take the traces out so the action buttons can mutate `app` freely.
    let traces = std::mem::take(&mut app.traces);
    egui::CentralPanel::default().show(ctx, |ui| {
        if traces.is_empty() {
            ui.add_space(20.0);
            ui.label(RichText::new("No specs found under .specs/.").color(DIM));
        } else {
            columns(app, ui, &traces);
        }
    });
    app.traces = traces;
}

fn columns(app: &mut HarnessApp, ui: &mut egui::Ui, traces: &[SpecTrace]) {
    ui.columns(2, |cols| {
        cols[0].push_id("col-spec", |ui| {
            ui.label(
                RichText::new(
                    "SPEC  —  one block per specification (requirements + design + tasks)",
                )
                .color(DIM)
                .strong(),
            );
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for t in traces {
                        spec_block(app, ui, t);
                        ui.add_space(10.0);
                    }
                });
        });

        cols[1].push_id("col-code", |ui| {
            ui.label(
                RichText::new("CODE  —  owned files, per spec")
                    .color(DIM)
                    .strong(),
            );
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for t in traces {
                        code_block(ui, t);
                        ui.add_space(10.0);
                    }
                });
        });
    });
}

/// One specification rendered as a single unit.
fn spec_block(app: &mut HarnessApp, ui: &mut egui::Ui, t: &SpecTrace) {
    let name = t.name.clone();
    ui.group(|ui| {
        // Header: spec name + sync badge.
        ui.horizontal(|ui| {
            ui.heading(RichText::new(&t.name).color(ACCENT));
            let st = t.sync.state();
            ui.label(
                RichText::new(format!("● {}", st.label()))
                    .color(sync_color(st))
                    .strong(),
            );
        });

        // Generation + sync detail.
        ui.horizontal(|ui| {
            ui.label(RichText::new("generation").color(DIM));
            ui.add(
                egui::ProgressBar::new(t.gen.ratio())
                    .desired_width(170.0)
                    .text(format!("{}/{} tasks", t.gen.done, t.gen.total())),
            );
            if t.gen.blocked > 0 {
                ui.label(RichText::new(format!("✗ {} blocked", t.gen.blocked)).color(FAIL));
            }
        });
        if t.sync.stale_inputs {
            ui.label(RichText::new("spec edited since baseline").color(WARN));
        }
        if !t.sync.drifted_files.is_empty() {
            ui.label(
                RichText::new(format!("{} file(s) drifted", t.sync.drifted_files.len()))
                    .color(FAIL),
            );
        }
        if !t.sync.missing_files.is_empty() {
            ui.label(RichText::new(format!("{} missing", t.sync.missing_files.len())).color(FAIL));
        }

        // Forward-sync actions (spec → code) + baseline.
        let running = app.is_running();
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(!running, |ui| {
                if ui
                    .button("Check")
                    .on_hover_text("harness check <spec> — report drift")
                    .clicked()
                {
                    app.launch(vec!["check".into(), name.clone()]);
                }
                if ui
                    .button(RichText::new("Build ▶").color(OK))
                    .on_hover_text("harness build <spec> — make code conform to the spec")
                    .clicked()
                {
                    app.launch(vec!["build".into(), name.clone()]);
                }
                if ui
                    .button(RichText::new("Rebuild").color(WARN))
                    .on_hover_text("harness rebuild <spec> --force — destructive re-render")
                    .clicked()
                {
                    app.confirm_rebuild = Some(name.clone());
                }
                if ui
                    .button("Accept baseline")
                    .on_hover_text("harness check <spec> --accept — record current state")
                    .clicked()
                {
                    app.launch(vec!["check".into(), name.clone(), "--accept".into()]);
                }
            });
        });
        if app.confirm_rebuild.as_deref() == Some(name.as_str()) {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Rebuild re-renders owned files (destructive).").color(WARN),
                );
                if ui
                    .button(RichText::new("Confirm rebuild").color(FAIL))
                    .clicked()
                {
                    app.launch(vec!["rebuild".into(), name.clone(), "--force".into()]);
                    app.confirm_rebuild = None;
                }
                if ui.button("Cancel").clicked() {
                    app.confirm_rebuild = None;
                }
            });
        }

        ui.separator();

        // Requirements.
        ui.label(RichText::new("Requirements").color(DIM).strong());
        if t.requirements.is_empty() {
            ui.label(RichText::new("— none (1-requirements.json)").color(DIM));
        }
        for r in &t.requirements {
            let covered = !t.tasks_for_requirement(&r.id).is_empty();
            let text = r
                .text
                .clone()
                .or_else(|| r.response.clone())
                .unwrap_or_default();
            let mut label = format!("• {}  {}", r.id, text);
            if !covered {
                label.push_str("  ⚠ no task");
            }
            ui.label(RichText::new(label));
        }

        ui.add_space(4.0);
        // Design.
        ui.label(RichText::new("Design").color(DIM).strong());
        if t.design.is_empty() {
            ui.label(RichText::new("— none (2-design.md)").color(DIM));
        } else {
            for line in t.design.lines() {
                ui.label(RichText::new(line).color(Color32::GRAY).monospace());
            }
        }

        ui.add_space(4.0);
        // Tasks.
        ui.label(RichText::new("Tasks").color(DIM).strong());
        if t.tasks.is_empty() {
            ui.label(RichText::new("— none (3-tasks.jsonl)").color(DIM));
        }
        for task in &t.tasks {
            let mut label = format!(
                "{} {}  {}",
                crate::status_glyph(&task.status),
                task.id,
                task.title
            );
            if !task.phases.is_empty() {
                label.push_str(&format!(
                    "  [{}/{}]",
                    task.completed_phases.len(),
                    task.phases.len()
                ));
            }
            if task.attempts > 0 {
                label.push_str(&format!("  ×{}", task.attempts));
            }
            ui.label(RichText::new(label).color(crate::status_color(&task.status)));
        }
    });
}

/// One spec's owned code files (the right side of its boundary).
fn code_block(ui: &mut egui::Ui, t: &SpecTrace) {
    ui.group(|ui| {
        ui.label(RichText::new(&t.name).color(ACCENT).strong());
        if t.owned_files.is_empty() {
            ui.label(RichText::new("no owned files (set `owns` globs in the spec)").color(DIM));
        }
        for f in &t.owned_files {
            let (glyph, dcolor) = match f.drift {
                FileDrift::Clean => ("●", OK),
                FileDrift::Drifted => ("✱", FAIL),
                FileDrift::Missing => ("✗", FAIL),
                FileDrift::Unrecorded => ("○", WARN),
            };
            let color = if f.drift == FileDrift::Clean {
                DIM
            } else {
                dcolor
            };
            ui.label(RichText::new(format!("{glyph} {}", f.path)).color(color));
        }
    });
}

fn action_log(app: &mut HarnessApp, ui: &mut egui::Ui) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("ACTION OUTPUT").color(DIM).strong());
        if app.is_running() {
            ui.spinner();
            ui.label(RichText::new("running…").color(ACCENT));
        } else if let Some(code) = app.last_exit {
            let c = if code == 0 { OK } else { FAIL };
            ui.label(RichText::new(format!("last exit: {code}")).color(c));
        }
    });
    if app.log.is_empty() {
        ui.label(
            RichText::new(
                "Run Check / Build / Rebuild / Accept on a spec to drive spec → code sync.",
            )
            .color(DIM),
        );
        return;
    }
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for line in &app.log {
                ui.label(RichText::new(line).monospace());
            }
        });
}
