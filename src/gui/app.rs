use crate::chat::{generate_answer_stub, janet_filter};
use crate::homework_pack::{
    apply_pack_policy, create_pack_multi, export_pack_template, find_latest_pack,
    load_pack_from_file, load_submission_summaries, save_submission_with_answers,
    HomeworkAssignment, HomeworkPack, SubmissionSummary,
};
use crate::modules::{load_modules, role_allowed, LoadedModule, ModuleEntry};
use crate::settings::{save_settings, Settings};
use crate::theme::{
    apply_theme, ensure_theme_files, load_presets, load_theme, save_theme, ThemeConfig,
};
use eframe::{
    egui::{
        self, menu, Align, CentralPanel, Context, Layout, ProgressBar, RichText, ScrollArea,
        TopBottomPanel,
    },
    App, CreationContext,
};
use rfd::FileDialog;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
struct AssignmentDraft {
    id: String,
    title: String,
    subject: String,
    year_level: String,
    due_at: String,
    instructions_md: String,
    allow_games: bool,
    allow_ai_premark: bool,
    max_score: String,
}

#[derive(Debug, Clone)]
struct StudentScore {
    #[allow(dead_code)]
    student_id: String,
    student_name: String,
    subject: String,
    score: f32, // 0-100
}

#[derive(Debug, Clone)]
struct SubmissionRow {
    #[allow(dead_code)]
    assignment_id: String,
    assignment_title: String,
    student_id: String,
    student_name: String,
    subject: String,
    score: String,
    feedback: String,
    #[allow(dead_code)]
    submitted_at: String,
}

#[derive(Debug, Clone)]
enum TabKind {
    Home,
    Chat,
    Settings,
    Module {
        module: LoadedModule,
        cached_text: Option<String>,
    },
}

#[derive(Debug, Clone)]
struct Tab {
    id: usize,
    title: String,
    kind: TabKind,
    closable: bool,
    key: String,
}

pub struct ChattyApp {
    pub settings: Settings,
    base_path: PathBuf,
    modules: Vec<LoadedModule>,
    tabs: Vec<Tab>,
    active_tab: usize,
    next_tab_id: usize,
    chat_input: String,
    chat_log: Vec<(String, String)>,
    theme: ThemeConfig,
    presets: Vec<ThemeConfig>,
    allow_external_process: bool,
    current_pack: Option<HomeworkPack>,
    submissions: Vec<SubmissionSummary>,
    selected_assignment: Option<String>,
    submission_text: String,
    draft_assignments: Vec<HomeworkAssignment>,
    draft_input: AssignmentDraft,
    selected_students: HashSet<String>,
    assignment_filter: Option<String>,
    subject_filter: Option<String>,
    submission_attachments: Vec<String>,
}

impl ChattyApp {
    pub fn new(
        cc: &CreationContext<'_>,
        base_path: PathBuf,
        settings: Settings,
    ) -> io::Result<Self> {
        ensure_theme_files(&base_path)?;
        let presets = load_presets(&base_path);
        let theme = load_theme(&base_path, settings.ui.last_theme.as_deref());
        apply_theme(&theme, &cc.egui_ctx);

        let modules = load_modules(&base_path).unwrap_or_default();
        let pack = find_latest_pack(&base_path)
            .ok()
            .flatten()
            .map(|(_p, pack)| pack);
        let submissions = load_submission_summaries(&base_path).unwrap_or_default();
        let initial_selected = pack
            .as_ref()
            .and_then(|p| p.assignments.first().map(|a| a.id.clone()));

        Ok(Self {
            settings,
            base_path,
            modules,
            tabs: vec![
                Tab {
                    id: 0,
                    title: "Home".to_string(),
                    kind: TabKind::Home,
                    closable: false,
                    key: "home".to_string(),
                },
                Tab {
                    id: 1,
                    title: "Chat".to_string(),
                    kind: TabKind::Chat,
                    closable: false,
                    key: "chat".to_string(),
                },
            ],
            active_tab: 0,
            next_tab_id: 2,
            chat_input: String::new(),
            chat_log: Vec::new(),
            theme,
            presets,
            allow_external_process: false,
            current_pack: pack,
            submissions,
            selected_assignment: initial_selected,
            submission_text: String::new(),
            draft_assignments: Vec::new(),
            draft_input: AssignmentDraft {
                id: "hw-001".to_string(),
                title: "Homework title".to_string(),
                subject: "General".to_string(),
                year_level: "7".to_string(),
                due_at: "".to_string(),
                instructions_md: "Add instructions here.".to_string(),
                allow_games: false,
                allow_ai_premark: true,
                max_score: "100".to_string(),
            },
            selected_students: HashSet::new(),
            assignment_filter: None,
            subject_filter: None,
            submission_attachments: Vec::new(),
        })
    }

    fn reload_modules(&mut self) {
        self.modules = load_modules(&self.base_path).unwrap_or_default();
    }

    fn switch_theme(&mut self, name: &str, ctx: &Context) {
        self.theme = load_theme(&self.base_path, Some(name));
        apply_theme(&self.theme, ctx);
        self.settings.ui.last_theme = Some(self.theme.name.clone());
        let _ = save_theme(&self.base_path, &self.theme);
        let _ = save_settings(&self.settings, &self.base_path);
    }

    fn resync_homework(&mut self) {
        self.current_pack = find_latest_pack(&self.base_path)
            .ok()
            .flatten()
            .map(|(_p, pack)| pack);
        self.submissions = load_submission_summaries(&self.base_path).unwrap_or_default();
    }

    fn open_or_focus_tab(&mut self, key: &str, builder: impl FnOnce(&mut Self) -> Tab) {
        if let Some(idx) = self.tabs.iter().position(|t| t.key == key) {
            self.active_tab = idx;
            return;
        }
        let mut tab = builder(self);
        tab.id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
    }

    fn open_module_tab(&mut self, module: &LoadedModule) {
        let key = format!("module:{}", module.manifest.id);
        let m = module.clone();
        let tab_key = key.clone();
        self.open_or_focus_tab(&key, |_app| Tab {
            id: 0,
            title: m.manifest.title.clone(),
            kind: TabKind::Module {
                module: m,
                cached_text: None,
            },
            closable: true,
            key: tab_key,
        });
    }

    fn close_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() && self.tabs[idx].closable {
            self.tabs.remove(idx);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len().saturating_sub(1);
            }
        }
    }

    fn render_menu_bar(&mut self, ctx: &Context, ui: &mut egui::Ui) {
        menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Reload modules").clicked() {
                    self.reload_modules();
                    ui.close_menu();
                }
                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("View", |ui| {
                let preset_names: Vec<String> =
                    self.presets.iter().map(|p| p.name.clone()).collect();
                for name in preset_names {
                    let selected = self.theme.name == name;
                    if ui.selectable_label(selected, name.clone()).clicked() {
                        self.switch_theme(&name, ctx);
                        ui.close_menu();
                    }
                }
            });

            ui.menu_button("Modules", |ui| {
                let current_role = "teacher"; // placeholder until user role switching exists
                if self.modules.is_empty() {
                    ui.label("No modules found.");
                }
                let modules = self.modules.clone();
                for module in modules {
                    if !role_allowed(&module.manifest, current_role) {
                        continue;
                    }
                    if ui.button(module.manifest.title.clone()).clicked() {
                        self.open_module_tab(&module);
                        ui.close_menu();
                    }
                }
            });

            ui.menu_button("Tools", |ui| {
                ui.add_enabled(false, egui::Label::new("Coming soon"));
            });

            ui.menu_button("Settings", |ui| {
                if ui.button("Open settings tab").clicked() {
                    self.open_or_focus_tab("settings", |_app| Tab {
                        id: 0,
                        title: "Settings".to_string(),
                        kind: TabKind::Settings,
                        closable: true,
                        key: "settings".to_string(),
                    });
                    ui.close_menu();
                }
            });

            ui.menu_button("Help", |ui| {
                ui.label("Chatty-EDU v0.2 shell (egui)");
                ui.label(format!("Base path: {}", self.base_path.display()));
            });
        });
    }

    fn render_tab_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let mut to_close: Option<usize> = None;
            for (idx, tab) in self.tabs.iter().enumerate() {
                let active = idx == self.active_tab;
                ui.horizontal(|ui| {
                    if ui.selectable_label(active, tab.title.clone()).clicked() {
                        self.active_tab = idx;
                    }
                    if tab.closable {
                        if ui.button("x").clicked() {
                            to_close = Some(idx);
                        }
                    }
                });
            }

            if let Some(idx) = to_close {
                self.close_tab(idx);
            }
        });
    }

    fn render_home(&mut self, ui: &mut egui::Ui) {
        ui.heading("Home");
        ui.label(format!("Base path: {}", self.base_path.display()));
        ui.label(format!("Teacher mode: {}", self.settings.teacher_mode));
        ui.label(format!("Available modules: {}", self.modules.len()));
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("Rescan packs + submissions").clicked() {
                self.resync_homework();
            }
            if ui.button("Export pack template").clicked() {
                match export_pack_template(
                    &self.base_path,
                    "school",
                    &self.settings.student.class_id,
                ) {
                    Ok(path) => {
                        let _ = ui.label(format!("Template at {}", path.display()));
                    }
                    Err(e) => {
                        let _ = ui.label(format!("Failed: {e}"));
                    }
                };
            }
            if ui.button("Import pack file...").clicked() {
                if let Some(file) = FileDialog::new().add_filter("json", &["json"]).pick_file() {
                    let dest_dir = self.base_path.join("homework").join("assigned");
                    let _ = fs::create_dir_all(&dest_dir);
                    let dest = dest_dir.join(
                        file.file_name()
                            .unwrap_or_else(|| std::ffi::OsStr::new("homework_pack_import.json")),
                    );
                    if let Err(e) = fs::copy(&file, &dest) {
                        let _ = ui.label(format!("Import failed: {e}"));
                    } else if let Ok(pack) = load_pack_from_file(&dest) {
                        apply_pack_policy(&mut self.settings, &pack);
                        let _ = save_settings(&self.settings, &self.base_path);
                        self.current_pack = Some(pack);
                        self.resync_homework();
                        let _ = ui.label(format!("Imported to {}", dest.display()));
                    }
                }
            }
        });

        ui.separator();
        ui.heading("Pack builder (teacher)");
        ui.label("Build a multi-assignment pack to share via the portal.");
        ui.horizontal(|ui| {
            ui.label("Assign ID");
            ui.text_edit_singleline(&mut self.draft_input.id);
            ui.label("Title");
            ui.text_edit_singleline(&mut self.draft_input.title);
        });
        ui.horizontal(|ui| {
            ui.label("Subject");
            ui.text_edit_singleline(&mut self.draft_input.subject);
            ui.label("Year");
            ui.text_edit_singleline(&mut self.draft_input.year_level);
            ui.label("Due at");
            ui.text_edit_singleline(&mut self.draft_input.due_at);
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.draft_input.allow_games, "Allow games");
            ui.checkbox(&mut self.draft_input.allow_ai_premark, "Allow AI premark");
            ui.label("Max score");
            ui.text_edit_singleline(&mut self.draft_input.max_score);
        });
        ui.label("Instructions");
        ui.text_edit_multiline(&mut self.draft_input.instructions_md);

        ui.horizontal(|ui| {
            if ui.button("Add assignment to pack").clicked() {
                if !self.draft_input.id.trim().is_empty() {
                    let max_score = if self.draft_input.max_score.trim().is_empty() {
                        None
                    } else {
                        self.draft_input.max_score.trim().parse().ok()
                    };
                    let assignment = HomeworkAssignment {
                        id: self.draft_input.id.trim().to_string(),
                        title: self.draft_input.title.trim().to_string(),
                        subject: self.draft_input.subject.trim().to_string(),
                        year_level: self.draft_input.year_level.trim().to_string(),
                        due_at: if self.draft_input.due_at.trim().is_empty() {
                            None
                        } else {
                            Some(self.draft_input.due_at.trim().to_string())
                        },
                        instructions_md: self.draft_input.instructions_md.clone(),
                        attachments: vec![],
                        allow_games: self.draft_input.allow_games,
                        allow_ai_premark: self.draft_input.allow_ai_premark,
                        max_score,
                    };
                    self.draft_assignments.push(assignment);
                    self.draft_input.id = format!("hw-{:03}", self.draft_assignments.len() + 1);
                }
            }

            if ui.button("Clear draft list").clicked() {
                self.draft_assignments.clear();
            }

            if ui
                .add_enabled(
                    !self.draft_assignments.is_empty(),
                    egui::Button::new("Export pack"),
                )
                .clicked()
            {
                let school_id = "school";
                let class_id = &self.settings.student.class_id;
                match create_pack_multi(
                    &self.base_path,
                    school_id,
                    class_id,
                    self.draft_assignments.clone(),
                ) {
                    Ok(path) => {
                        let _ = ui.label(format!("Pack saved to {}", path.display()));
                        self.resync_homework();
                        self.draft_assignments.clear();
                    }
                    Err(e) => {
                        let _ = ui.label(format!("Failed: {e}"));
                    }
                }
            }
        });

        if !self.draft_assignments.is_empty() {
            ui.label(format!(
                "Assignments in pack: {}",
                self.draft_assignments.len()
            ));
        }

        if let Some(pack) = &self.current_pack {
            ui.separator();
            ui.label(format!(
                "Latest homework pack: {} (class {}) assignments: {}",
                pack.school_id,
                pack.class_id,
                pack.assignments.len()
            ));
            ui.horizontal(|ui| {
                ui.label("Assignment filter:");
                let current = self
                    .assignment_filter
                    .clone()
                    .unwrap_or_else(|| "All".to_string());
                egui::ComboBox::from_id_source("assignment_filter")
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(self.assignment_filter.is_none(), "All")
                            .clicked()
                        {
                            self.assignment_filter = None;
                        }
                        for a in &pack.assignments {
                            if ui
                                .selectable_label(
                                    self.assignment_filter.as_ref() == Some(&a.id),
                                    format!("{} - {}", a.id, a.title),
                                )
                                .clicked()
                            {
                                self.assignment_filter = Some(a.id.clone());
                            }
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Subject filter:");
                let subjects: Vec<String> = pack
                    .assignments
                    .iter()
                    .map(|a| a.subject.clone())
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect();
                let current = self
                    .subject_filter
                    .clone()
                    .unwrap_or_else(|| "All".to_string());
                egui::ComboBox::from_id_source("subject_filter")
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(self.subject_filter.is_none(), "All")
                            .clicked()
                        {
                            self.subject_filter = None;
                        }
                        for subj in subjects {
                            if ui
                                .selectable_label(
                                    self.subject_filter.as_ref() == Some(&subj),
                                    subj.clone(),
                                )
                                .clicked()
                            {
                                self.subject_filter = Some(subj.clone());
                            }
                        }
                    });
            });
            for a in &pack.assignments {
                if let Some(filter) = &self.assignment_filter {
                    if &a.id != filter {
                        continue;
                    }
                }
                if let Some(subj) = &self.subject_filter {
                    if &a.subject != subj {
                        continue;
                    }
                }
                let selected = self
                    .selected_assignment
                    .as_ref()
                    .map(|s| s == &a.id)
                    .unwrap_or(false);
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(selected, format!("{} - {}", a.id, a.title))
                        .clicked()
                    {
                        self.selected_assignment = Some(a.id.clone());
                    }
                    ui.label(format!(
                        "Subject: {} | Due: {}",
                        a.subject,
                        a.due_at.as_deref().unwrap_or("-"),
                    ));
                    if !a.allow_games {
                        ui.colored_label(egui::Color32::YELLOW, "Games off");
                    }
                });
                ui.label(format!("Instructions: {}", a.instructions_md));
                ui.separator();
            }

            ui.heading("Submit work");
            ui.label("Type your work and export a submission file to upload via the portal.");
            ui.add(
                egui::TextEdit::multiline(&mut self.submission_text)
                    .hint_text("Your answers, notes, or summary..."),
            );
            ui.horizontal(|ui| {
                if ui.button("Add attachments...").clicked() {
                    if let Some(files) = FileDialog::new().pick_files() {
                        for f in files {
                            if let Some(p) = f.to_str() {
                                self.submission_attachments.push(p.to_string());
                            }
                        }
                    }
                }
                if ui.button("Clear attachments").clicked() {
                    self.submission_attachments.clear();
                }
            });
            if !self.submission_attachments.is_empty() {
                ui.label("Attachments:");
                let mut to_remove: Option<usize> = None;
                for (idx, path) in self.submission_attachments.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{path}"));
                        if ui.small_button("x").clicked() {
                            to_remove = Some(idx);
                        }
                    });
                }
                if let Some(idx) = to_remove {
                    self.submission_attachments.remove(idx);
                }
            }
            let disabled = self.selected_assignment.is_none();
            let assign = self.selected_assignment.clone();
            if ui
                .add_enabled(!disabled, egui::Button::new("Export submission file"))
                .clicked()
            {
                if let Some(id) = assign {
                    match save_submission_with_answers(
                        &self.base_path,
                        &self.settings,
                        &id,
                        &self.submission_text,
                        &self.submission_attachments,
                    ) {
                        Ok(path) => {
                            let _ = ui.label(format!("Saved to {}", path.display()));
                            self.submission_text.clear();
                            self.submission_attachments.clear();
                            self.resync_homework();
                        }
                        Err(e) => {
                            let _ = ui.label(format!("Failed: {e}"));
                        }
                    }
                }
            }
        } else {
            ui.label(
                "No homework pack found. Drop a homework_pack*.json into homework/assigned/ and click Rescan.",
            );
        }

        if !self.submissions.is_empty() {
            ui.separator();
            ui.heading("Submissions found locally");
            ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                for row in self.submission_rows() {
                    let label = format!(
                        "{} ({}) - {} ({}) | subj: {} | score: {} | {}",
                        row.assignment_title,
                        row.assignment_id,
                        row.student_name,
                        row.student_id,
                        row.subject,
                        row.score,
                        row.feedback
                    );
                    ui.label(label).on_hover_text(format!(
                        "Assignment ID: {} | Student ID: {} | Submitted: {}",
                        row.assignment_id, row.student_id, row.submitted_at
                    ));
                }
            });
        }
    }

    fn render_chat(&mut self, ui: &mut egui::Ui) {
        ui.heading("Chat");
        ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            for (sender, msg) in &self.chat_log {
                ui.label(format!("{}: {}", sender, msg));
            }
        });
    }

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.label("Student profile");
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut self.settings.student.student_name);
        });
        ui.horizontal(|ui| {
            ui.label("Student ID");
            ui.text_edit_singleline(&mut self.settings.student.student_id);
        });
        ui.horizontal(|ui| {
            ui.label("Class ID");
            ui.text_edit_singleline(&mut self.settings.student.class_id);
        });
        ui.separator();
        ui.checkbox(
            &mut self.settings.janet.enabled,
            "Enable Janet safety filter",
        );
        ui.checkbox(
            &mut self.settings.janet.block_swears,
            "Block swears and rude words",
        );
        ui.checkbox(
            &mut self.settings.janet.block_mature_topics,
            "Block mature topics",
        );
        ui.separator();
        ui.checkbox(&mut self.settings.game.enabled, "Enable games");
        ui.checkbox(
            &mut self.settings.game.games_in_class_allowed,
            "Allow games in class",
        );
        if ui.button("Save settings").clicked() {
            let _ = save_settings(&self.settings, &self.base_path);
            ui.label("Saved");
        }
    }

    fn render_homework_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Homework dashboard");
        if let Some(pack) = &self.current_pack {
            ui.label(format!(
                "Class: {} | Assignments: {}",
                pack.class_id,
                pack.assignments.len()
            ));
        } else {
            ui.label("No pack loaded yet. Import a pack to see class metrics.");
        }

        let all_entries = self.score_entries();
        let focused_entries: Vec<StudentScore> = if self.selected_students.is_empty() {
            all_entries.clone()
        } else {
            all_entries
                .into_iter()
                .filter(|s| self.selected_students.contains(&s.student_name))
                .collect()
        };

        if focused_entries.is_empty() {
            ui.label("No submissions found yet.");
        } else {
            let (class_avg, per_student_avg, per_subject_avg) = aggregate_scores(&focused_entries);
            ui.separator();
            ui.label("Class / selection average");
            ui.add(
                ProgressBar::new(class_avg / 100.0)
                    .fill(score_color(class_avg))
                    .text(format!("{:.1} / 100", class_avg)),
            );

            ui.horizontal(|ui| {
                ui.label("Students:");
                if ui.button("Clear selection").clicked() {
                    self.selected_students.clear();
                }
            });
            ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                for (name, avg) in &per_student_avg {
                    let selected = self.selected_students.contains(name);
                    let label = format!("{name} ({avg:.1})");
                    if ui.selectable_label(selected, label).clicked() {
                        if selected {
                            self.selected_students.remove(name);
                        } else {
                            self.selected_students.insert(name.clone());
                        }
                    }
                }
            });

            ui.separator();
            ui.label("Subject metrics");
            for (subj, score) in &per_subject_avg {
                ui.horizontal(|ui| {
                    ui.label(subj);
                    ui.add(
                        ProgressBar::new(*score / 100.0)
                            .fill(score_color(*score))
                            .text(format!("{score:.1}")),
                    );
                });
            }
        }

        if !self.submissions.is_empty() {
            ui.separator();
            ui.heading("Submissions found locally");
            ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                for row in self.submission_rows() {
                    let label = format!(
                        "{} ({}) - {} ({}) | subj: {} | score: {} | {}",
                        row.assignment_title,
                        row.assignment_id,
                        row.student_name,
                        row.student_id,
                        row.subject,
                        row.score,
                        row.feedback
                    );
                    ui.label(label).on_hover_text(format!(
                        "Assignment ID: {} | Student ID: {} | Submitted: {}",
                        row.assignment_id, row.student_id, row.submitted_at
                    ));
                }
            });
        }
    }

    fn render_module_tab(&mut self, ui: &mut egui::Ui, tab_idx: usize) {
        let Some(tab) = self.tabs.get_mut(tab_idx) else {
            return;
        };
        let TabKind::Module {
            module,
            cached_text,
        } = &mut tab.kind
        else {
            return;
        };

        ui.heading(&module.manifest.title);
        if let Some(desc) = &module.manifest.description {
            ui.label(desc);
        }
        ui.separator();

        match &module.manifest.entry {
            ModuleEntry::BuiltinPanel { target } => match target.as_str() {
                "homework_dashboard" => self.render_homework_dashboard(ui),
                _ => {
                    ui.label(format!("Builtin panel stub: {}", target));
                }
            },
            ModuleEntry::Markdown { path } => {
                if cached_text.is_none() {
                    let full_path = module.folder.join(path);
                    *cached_text = fs::read_to_string(&full_path).ok();
                }
                if let Some(text) = cached_text {
                    render_markdown(ui, text);
                } else {
                    ui.label("Could not load markdown file.");
                }
            }
            ModuleEntry::StaticHtml { path } => {
                ui.label(format!("Static HTML module (not rendered yet): {}", path));
            }
            ModuleEntry::ExternalProcess { command, args } => {
                if self.allow_external_process {
                    ui.label(format!(
                        "External process would run: {} {:?}",
                        command, args
                    ));
                    ui.label("Process launching is stubbed for safety.");
                } else {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        "External processes are disabled in safe mode.",
                    );
                }
            }
        }
    }

    fn submission_rows(&self) -> Vec<SubmissionRow> {
        let mut rows = Vec::new();
        for s in &self.submissions {
            let (title, subject) = self
                .current_pack
                .as_ref()
                .and_then(|p| {
                    p.assignments
                        .iter()
                        .find(|a| a.id == s.assignment_id)
                        .map(|a| (a.title.clone(), a.subject.clone()))
                })
                .unwrap_or_else(|| ("Assignment".to_string(), "General".to_string()));
            let score = s
                .ai_score
                .or(s.score)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string());
            let feedback = s
                .ai_feedback
                .clone()
                .unwrap_or_else(|| "No AI feedback".to_string());
            rows.push(SubmissionRow {
                assignment_id: s.assignment_id.clone(),
                assignment_title: title,
                student_id: s.student_id.clone(),
                student_name: s.student_name.clone(),
                subject,
                score,
                feedback,
                submitted_at: s.submitted_at.clone(),
            });
        }
        rows
    }

    fn score_entries(&self) -> Vec<StudentScore> {
        self.submissions
            .iter()
            .map(|s| {
                let subject = self
                    .current_pack
                    .as_ref()
                    .and_then(|p| {
                        p.assignments
                            .iter()
                            .find(|a| a.id == s.assignment_id)
                            .map(|a| a.subject.clone())
                    })
                    .unwrap_or_else(|| "General".to_string());
                let score_val = s.ai_score.or(s.score).unwrap_or(0) as f32;
                StudentScore {
                    student_id: s.student_id.clone(),
                    student_name: s.student_name.clone(),
                    subject,
                    score: score_val,
                }
            })
            .collect()
    }

    fn handle_chat_send(&mut self) {
        if self.chat_input.trim().is_empty() {
            return;
        }
        let user_msg = self.chat_input.trim().to_string();
        self.chat_log.push(("You".to_string(), user_msg.clone()));
        let raw_answer = generate_answer_stub(&user_msg);
        let safe = janet_filter(&self.settings.janet, &raw_answer, &user_msg);
        self.chat_log.push(("Chatty".to_string(), safe));
        self.chat_input.clear();
    }
}
fn render_markdown(ui: &mut egui::Ui, text: &str) {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            ui.heading(trimmed.trim_start_matches("# ").trim());
        } else if trimmed.starts_with("## ") {
            ui.label(RichText::new(trimmed.trim_start_matches("## ").trim()).strong());
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            ui.label(format!("* {}", trimmed[2..].trim()));
        } else if trimmed.is_empty() {
            ui.add_space(6.0);
        } else {
            ui.label(trimmed);
        }
    }
}

fn aggregate_scores(entries: &[StudentScore]) -> (f32, Vec<(String, f32)>, Vec<(String, f32)>) {
    let mut per_student: HashMap<String, Vec<f32>> = HashMap::new();
    let mut per_subject: HashMap<String, Vec<f32>> = HashMap::new();
    for e in entries {
        per_student
            .entry(e.student_name.clone())
            .or_default()
            .push(e.score);
        per_subject
            .entry(e.subject.clone())
            .or_default()
            .push(e.score);
    }

    let avg = |vals: &[f32]| -> f32 {
        if vals.is_empty() {
            0.0
        } else {
            vals.iter().copied().sum::<f32>() / vals.len() as f32
        }
    };

    let class_overall = avg(&entries.iter().map(|e| e.score).collect::<Vec<_>>());

    let mut per_student_avg: Vec<(String, f32)> =
        per_student.into_iter().map(|(k, v)| (k, avg(&v))).collect();
    per_student_avg.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut per_subject_avg: Vec<(String, f32)> =
        per_subject.into_iter().map(|(k, v)| (k, avg(&v))).collect();
    per_subject_avg.sort_by(|a, b| a.0.cmp(&b.0));

    (class_overall, per_student_avg, per_subject_avg)
}

fn score_color(score: f32) -> egui::Color32 {
    let t = (score / 100.0).clamp(0.0, 1.0);
    let r = ((1.0 - t) * 255.0) as u8;
    let g = (t * 200.0 + 55.0).min(255.0) as u8;
    egui::Color32::from_rgb(r, g, 64)
}

impl App for ChattyApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        apply_theme(&self.theme, ctx);

        TopBottomPanel::top("menu_bar").show(ctx, |ui| self.render_menu_bar(ctx, ui));
        TopBottomPanel::top("tabs").show(ctx, |ui| self.render_tab_bar(ui));

        CentralPanel::default().show(ctx, |ui| {
            if let Some(tab) = self.tabs.get(self.active_tab).cloned() {
                match tab.kind {
                    TabKind::Home => self.render_home(ui),
                    TabKind::Chat => self.render_chat(ui),
                    TabKind::Settings => self.render_settings(ui),
                    TabKind::Module { .. } => self.render_module_tab(ui, self.active_tab),
                }
            }
        });

        TopBottomPanel::bottom("chat_input").show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.label("Chat:");
                let input = ui.add(
                    egui::TextEdit::singleline(&mut self.chat_input)
                        .hint_text("Ask or type a command..."),
                );
                if input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.handle_chat_send();
                }
                if ui.button("Send").clicked() {
                    self.handle_chat_send();
                }
            });
        });
    }
}

pub fn launch_gui(base_path: PathBuf, settings: Settings) -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Chatty-EDU")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Chatty-EDU",
        native_options,
        Box::new(move |cc| {
            let app =
                ChattyApp::new(cc, base_path.clone(), settings.clone()).unwrap_or_else(|_| {
                    ChattyApp::new(cc, base_path.clone(), settings.clone())
                        .expect("Failed to start app")
                });
            Box::new(app)
        }),
    )
}
