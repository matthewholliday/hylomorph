//! Trace mode: a two-column **Spec ⟷ Code** view of one spec.
//!
//! The manifest tracks a single boundary: the spec inputs *as one unit*
//! (requirements + design + tasks, combined into one hash) versus the owned code
//! (hashed per file). So the layout is two columns — the spec stacked on the
//! left, code on the right — not a four-stage pipeline.
//!
//! Spec-as-source means sync is directional: spec → code (forward) is driven by
//! `build`/`rebuild`; the reverse is only *detected* as drift and resolved by
//! reverting code or accepting a new baseline. The controls here reflect that.

use std::collections::HashSet;

use eframe::egui;
use egui::{Color32, RichText};

use harness_core::trace::{FileDrift, SpecTrace, SyncState};

use crate::{HarnessApp, Selection, ACCENT, DIM, FAIL, OK, WARN};

/// Highlighted ids/paths across columns, derived from the current selection.
#[derive(Default)]
struct Hi {
    reqs: HashSet<String>,
    tasks: HashSet<String>,
    files: HashSet<String>,
}

fn compute_hi(t: &SpecTrace, sel: &Selection) -> Hi {
    let mut hi = Hi::default();
    match sel {
        Selection::None => {}
        Selection::Requirement(r) => {
            hi.reqs.insert(r.clone());
            for task in t.tasks_for_requirement(r) {
                hi.tasks.insert(task.id.clone());
                for f in &t.owned_files {
                    if SpecTrace::task_touches(task, &f.path) {
                        hi.files.insert(f.path.clone());
                    }
                }
            }
        }
        Selection::Task(id) => {
            hi.tasks.insert(id.clone());
            if let Some(task) = t.tasks.iter().find(|x| &x.id == id) {
                for r in t.requirements_for_task(task) {
                    hi.reqs.insert(r.id.clone());
                }
                for f in &t.owned_files {
                    if SpecTrace::task_touches(task, &f.path) {
                        hi.files.insert(f.path.clone());
                    }
                }
            }
        }
        Selection::File(p) => {
            hi.files.insert(p.clone());
            for task in &t.tasks {
                if SpecTrace::task_touches(task, p) {
                    hi.tasks.insert(task.id.clone());
                    for r in t.requirements_for_task(task) {
                        hi.reqs.insert(r.id.clone());
                    }
                }
            }
        }
    }
    hi
}

/// Decide an item's colour given the selection context.
/// `primary` = the clicked item; `related` = linked to it.
fn item_text(
    label: String,
    base: Color32,
    has_sel: bool,
    primary: bool,
    related: bool,
) -> RichText {
    if primary {
        RichText::new(label).color(ACCENT).strong()
    } else if !has_sel {
        RichText::new(label).color(base)
    } else if related {
        RichText::new(label).color(base).strong()
    } else {
        RichText::new(label).color(DIM)
    }
}

fn sync_color(s: SyncState) -> Color32 {
    match s {
        SyncState::Clean => OK,
        SyncState::Stale => WARN,
        SyncState::Drifted => FAIL,
        SyncState::Unrecorded => DIM,
    }
}

pub fn ui(app: &mut HarnessApp, ctx: &egui::Context) {
    app.ensure_trace();

    egui::TopBottomPanel::top("trace-controls").show(ctx, |ui| {
        controls(app, ui);
    });

    egui::TopBottomPanel::bottom("trace-log")
        .resizable(true)
        .default_height(120.0)
        .show(ctx, |ui| {
            action_log(app, ui);
        });

    // Take the trace out so column closures can mutate `app.selection` freely.
    let trace = app.trace.take();
    egui::CentralPanel::default().show(ctx, |ui| match &trace {
        Some(t) => columns(app, ui, t),
        None => {
            ui.add_space(20.0);
            ui.label(
                RichText::new("No spec selected, or no specs found under .specs/.").color(DIM),
            );
        }
    });
    app.trace = trace;
}

fn controls(app: &mut HarnessApp, ui: &mut egui::Ui) {
    ui.add_space(4.0);

    // Spec selector.
    let specs = app.specs();
    let mut sel = app.trace_spec.clone().unwrap_or_default();
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("trace-spec")
            .selected_text(if sel.is_empty() {
                "—".into()
            } else {
                sel.clone()
            })
            .show_ui(ui, |ui| {
                for s in &specs {
                    ui.selectable_value(&mut sel, s.clone(), s);
                }
            });
        ui.label(RichText::new("spec").color(DIM));
    });
    if !sel.is_empty() && app.trace_spec.as_deref() != Some(sel.as_str()) {
        app.trace_spec = Some(sel);
        app.selection = Selection::None;
        app.reload_trace();
    }

    // Pull display values out before borrowing app mutably for buttons.
    let info = app.trace.as_ref().map(|t| {
        (
            t.gen.done,
            t.gen.total(),
            t.gen.blocked,
            t.gen.ratio(),
            t.sync.state(),
            t.sync.drifted_files.len(),
            t.sync.missing_files.len(),
            t.sync.stale_inputs,
        )
    });

    if let Some((done, total, blocked, ratio, state, drift, missing, stale)) = info {
        ui.add_space(4.0);
        // Generation progress (spec → validated code).
        ui.horizontal(|ui| {
            ui.label(RichText::new("generation").color(DIM));
            ui.add(
                egui::ProgressBar::new(ratio)
                    .desired_width(220.0)
                    .text(format!("{done}/{total} tasks")),
            );
            if blocked > 0 {
                ui.label(RichText::new(format!("✗ {blocked} blocked")).color(FAIL));
            }
        });

        ui.add_space(2.0);
        // Spec → Code sync gutter.
        ui.horizontal(|ui| {
            ui.label(RichText::new("SPEC").strong());
            ui.label(RichText::new("──▶").color(DIM));
            ui.label(RichText::new("CODE").strong());
            ui.separator();
            ui.label(
                RichText::new(format!("● {}", state.label()))
                    .color(sync_color(state))
                    .strong(),
            );
            if stale {
                ui.label(RichText::new("spec edited since baseline").color(WARN));
            }
            if drift > 0 {
                ui.label(RichText::new(format!("{drift} file(s) drifted")).color(FAIL));
            }
            if missing > 0 {
                ui.label(RichText::new(format!("{missing} missing")).color(FAIL));
            }
        });
    }

    // Sync actions (forward: spec → code) + baseline.
    ui.add_space(4.0);
    let spec = app.trace_spec.clone();
    let running = app.is_running();
    ui.horizontal(|ui| {
        ui.add_enabled_ui(!running && spec.is_some(), |ui| {
            let s = spec.clone().unwrap_or_default();
            if ui
                .button("Check")
                .on_hover_text("harness check <spec> — report drift")
                .clicked()
            {
                app.launch(vec!["check".into(), s.clone()]);
            }
            if ui
                .button(RichText::new("Build ▶").color(OK))
                .on_hover_text("harness build <spec> — make code conform to the spec")
                .clicked()
            {
                app.launch(vec!["build".into(), s.clone()]);
            }
            if ui
                .button(RichText::new("Rebuild").color(WARN))
                .on_hover_text("harness rebuild <spec> --force — destructive re-render from spec")
                .clicked()
            {
                app.confirm_rebuild = true;
            }
            if ui
                .button("Accept baseline")
                .on_hover_text(
                    "harness check <spec> --accept — record current state as the baseline",
                )
                .clicked()
            {
                app.launch(vec!["check".into(), s.clone(), "--accept".into()]);
            }
        });
        if running {
            ui.spinner();
            ui.label(RichText::new("running…").color(ACCENT));
        } else if let Some(code) = app.last_exit {
            let c = if code == 0 { OK } else { FAIL };
            ui.label(RichText::new(format!("last exit: {code}")).color(c));
        }
    });

    // Destructive-action confirmation.
    if app.confirm_rebuild {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Rebuild re-renders owned files from the spec (destructive).")
                    .color(WARN),
            );
            if ui
                .button(RichText::new("Confirm rebuild").color(FAIL))
                .clicked()
            {
                if let Some(s) = spec.clone() {
                    app.launch(vec!["rebuild".into(), s, "--force".into()]);
                }
                app.confirm_rebuild = false;
            }
            if ui.button("Cancel").clicked() {
                app.confirm_rebuild = false;
            }
        });
    }
    ui.add_space(4.0);
}

fn action_log(app: &mut HarnessApp, ui: &mut egui::Ui) {
    ui.add_space(2.0);
    ui.label(RichText::new("ACTION OUTPUT").color(DIM).strong());
    if app.log.is_empty() {
        ui.label(
            RichText::new("Run Check / Build / Rebuild / Accept to drive spec → code sync.")
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

/// Two columns: the spec as one unit (requirements + design + tasks stacked) on
/// the left, the owned code on the right — matching the single hash boundary the
/// manifest actually tracks (combined spec inputs ⟷ per-file code).
fn columns(app: &mut HarnessApp, ui: &mut egui::Ui, t: &SpecTrace) {
    let hi = compute_hi(t, &app.selection);
    let has_sel = app.selection != Selection::None;

    ui.columns(2, |cols| {
        // ── Spec (requirements + design + tasks) ───────────────────────────
        cols[0].push_id("col-spec", |ui| {
            ui.label(
                RichText::new("SPEC  (requirements + design + tasks)")
                    .color(DIM)
                    .strong(),
            );
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Requirements.
                    ui.label(RichText::new("Requirements").color(ACCENT).strong());
                    if t.requirements.is_empty() {
                        ui.label(RichText::new("no 1-requirements.json").color(DIM));
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
                        let primary =
                            matches!(&app.selection, Selection::Requirement(x) if x == &r.id);
                        let related = hi.reqs.contains(&r.id);
                        let rt = item_text(label, ACCENT, has_sel, primary, related);
                        if ui.selectable_label(primary, rt).clicked() {
                            app.selection = if primary {
                                Selection::None
                            } else {
                                Selection::Requirement(r.id.clone())
                            };
                        }
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    // Design.
                    ui.label(RichText::new("Design").color(ACCENT).strong());
                    if t.design.is_empty() {
                        ui.label(RichText::new("no 2-design.md").color(DIM));
                    }
                    let highlight_reqs = has_sel && !hi.reqs.is_empty();
                    for line in t.design.lines() {
                        let mentions =
                            highlight_reqs && hi.reqs.iter().any(|id| line.contains(id.as_str()));
                        let color = if !highlight_reqs {
                            Color32::GRAY
                        } else if mentions {
                            ACCENT
                        } else {
                            DIM
                        };
                        ui.label(RichText::new(line).color(color).monospace());
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    // Tasks.
                    ui.label(RichText::new("Tasks").color(ACCENT).strong());
                    if t.tasks.is_empty() {
                        ui.label(RichText::new("no 3-tasks.jsonl").color(DIM));
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
                        let base = crate::status_color(&task.status);
                        let primary = matches!(&app.selection, Selection::Task(x) if x == &task.id);
                        let related = hi.tasks.contains(&task.id);
                        let rt = item_text(label, base, has_sel, primary, related);
                        if ui.selectable_label(primary, rt).clicked() {
                            app.selection = if primary {
                                Selection::None
                            } else {
                                Selection::Task(task.id.clone())
                            };
                        }
                    }
                });
        });

        // ── Code ───────────────────────────────────────────────────────────
        cols[1].push_id("col-code", |ui| {
            ui.label(RichText::new("CODE  (owned files)").color(DIM).strong());
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if t.owned_files.is_empty() {
                        ui.label(
                            RichText::new("no owned files (set `owns` globs in the spec)")
                                .color(DIM),
                        );
                    }
                    for f in &t.owned_files {
                        let (glyph, dcolor) = match f.drift {
                            FileDrift::Clean => ("●", OK),
                            FileDrift::Drifted => ("✱", FAIL),
                            FileDrift::Missing => ("✗", FAIL),
                            FileDrift::Unrecorded => ("○", WARN),
                        };
                        let label = format!("{glyph} {}", f.path);
                        let primary = matches!(&app.selection, Selection::File(x) if x == &f.path);
                        let related = hi.files.contains(&f.path);
                        // Drift colour wins unless dimmed by an unrelated selection.
                        let base = if f.drift == FileDrift::Clean {
                            DIM
                        } else {
                            dcolor
                        };
                        let rt = item_text(label, base, has_sel, primary, related);
                        if ui.selectable_label(primary, rt).clicked() {
                            app.selection = if primary {
                                Selection::None
                            } else {
                                Selection::File(f.path.clone())
                            };
                        }
                    }
                });
        });
    });
}
