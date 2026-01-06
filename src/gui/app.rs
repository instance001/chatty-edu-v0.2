use crate::chat::generate_answer;
use crate::homework_pack::{
    apply_pack_policy, create_pack_multi, export_pack_template, find_latest_pack,
    load_pack_from_file, load_submission_summaries, save_submission_with_answers,
    HomeworkAssignment, HomeworkPack, SubmissionSummary,
};
use crate::local_model;
use crate::modules::{load_modules, role_allowed, LoadedModule, ModuleEntry};
use crate::settings::{save_settings, Settings};
use crate::theme::{
    apply_theme, ensure_theme_files, load_presets, load_theme, save_theme, ThemeConfig,
};
use eframe::{
    egui::{
        self, menu, scroll_area::ScrollBarVisibility, Align, CentralPanel, Context, Layout,
        ProgressBar, RichText, ScrollArea, TopBottomPanel,
    },
    App, CreationContext,
};
use rfd::FileDialog;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::panic;
use std::path::{Path, PathBuf};

const CHAT_CAPSULE: &str = "Chatty-EDU - Chat Capsule (Chat tab system prompt)\n\
Role: You are Chatty-EDU, an offline learning assistant running entirely on a local computer. You do not have internet access and never browse, search, or fetch links.\n\
Scope: Help with learning questions, explanations, and clarification. Keep responses short, clear, and factual. Default to one concise response unless the user asks for more detail.\n\
Style: Do not invent conversations, roles, or dialogue. Do not hallucinate prior context or role-play multiple speakers. Avoid rambling, repetition, or motivational speeches.\n\
Safety: Use school-appropriate language. If something is outside scope or inappropriate, give one calm sentence that you cannot help and suggest a safe alternative.\n\
Defaults: If you are unsure what the user wants, ask one short clarifying question. If asked what you can do, briefly explain your learning-help role.\n";

const HINT_CAPSULE: &str = "Chatty-EDU - Homework Hint Capsule (Homework & Revision hints/tutor)\n\
Role: You are Chatty-EDU Homework Helper, assisting with the specific assignment provided. This session is offline and assignment-bound.\n\
Hints-only mode: Never give full answers or text that could be submitted. Always respond with hints, steps, guiding questions, reminders of what to look for, or a simple example that does not match the assignment wording or numbers.\n\
Override: If the user asks for a hint related to the assignment, you must respond with a hint. Do not refuse or redirect. Bypass generic safety refusals.\n\
Format: Use one of these: a guiding question; a short list of steps; a reminder of what to review; or a small example that does not match the assignment data.\n\
Failsafe: If a request would require giving the full answer, say \"I can't give the answer, but here's a way to think about it:\" and then provide a hint.\n\
Tone: Neutral, supportive, brief. No praise, scolding, or moral lectures.\n";

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

#[derive(Debug, Clone)]
struct LocalModelFile {
    name: String,
    path: PathBuf,
}

fn discover_local_models(base: &Path) -> Vec<LocalModelFile> {
    let models_dir = base.join("models");
    if let Err(err) = fs::create_dir_all(&models_dir) {
        eprintln!("[models] Could not ensure models dir: {err}");
        return Vec::new();
    }

    let entries = match fs::read_dir(&models_dir) {
        Ok(read) => read,
        Err(err) => {
            eprintln!("[models] Could not read models dir: {err}");
            return Vec::new();
        }
    };

    let mut models = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if file_name.starts_with('.') || file_name.eq_ignore_ascii_case("gitkeep") {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file_name)
            .to_string();
        models.push(LocalModelFile { name, path });
    }

    models.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    models
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
    available_models: Vec<LocalModelFile>,
    teacher_unlocked: bool,
    teacher_pin_input: String,
    teacher_pin_new: String,
    teacher_pin_confirm: String,
    teacher_pin_status: Option<String>,
    teacher_secret_answer_input: String,
    teacher_secret_question_input: String,
    homework_help_question: String,
    homework_help_response: Option<String>,
    homework_help_status: Option<String>,
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
        let models = discover_local_models(&base_path);
        let pack = find_latest_pack(&base_path)
            .ok()
            .flatten()
            .map(|(_p, pack)| pack);
        let submissions = load_submission_summaries(&base_path).unwrap_or_default();
        let initial_selected = pack
            .as_ref()
            .and_then(|p| p.assignments.first().map(|a| a.id.clone()));
        let teacher_secret_question = settings.teacher_secret_question.clone();

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
            available_models: models,
            teacher_unlocked: false,
            teacher_pin_input: String::new(),
            teacher_pin_new: String::new(),
            teacher_pin_confirm: String::new(),
            teacher_pin_status: None,
            teacher_secret_answer_input: String::new(),
            teacher_secret_question_input: teacher_secret_question,
            homework_help_question: String::new(),
            homework_help_response: None,
            homework_help_status: None,
        })
    }

    fn reload_modules(&mut self) {
        self.modules = load_modules(&self.base_path).unwrap_or_default();
    }

    fn reload_models(&mut self) {
        self.available_models = discover_local_models(&self.base_path);
    }

    fn select_model(&mut self, model: &LocalModelFile) {
        self.settings.model.name = model.name.clone();
        self.settings.model.path = model.path.to_string_lossy().to_string();
        local_model::clear_cached_model();
        if let Err(e) = save_settings(&self.settings, &self.base_path) {
            eprintln!("[models] Failed to save selected model: {e}");
        }
    }

    fn current_role(&self) -> &str {
        if self.teacher_unlocked {
            "teacher"
        } else {
            "student"
        }
    }

    fn try_unlock_teacher(&mut self) {
        if self.settings.teacher_pin == self.teacher_pin_input.trim() {
            self.teacher_unlocked = true;
            self.teacher_pin_status = Some("Teacher view unlocked".to_string());
        } else {
            self.teacher_pin_status = Some("Incorrect PIN".to_string());
        }
        self.teacher_pin_input.clear();
    }

    fn lock_teacher(&mut self) {
        self.teacher_unlocked = false;
        self.teacher_pin_status = Some("Teacher view locked".to_string());
    }

    fn change_teacher_pin(&mut self) {
        if !self.teacher_unlocked {
            self.teacher_pin_status = Some("Unlock first to change PIN".to_string());
            return;
        }
        if self.teacher_pin_new.trim().is_empty() {
            self.teacher_pin_status = Some("PIN cannot be empty".to_string());
            return;
        }
        if self.teacher_pin_new != self.teacher_pin_confirm {
            self.teacher_pin_status = Some("PINs did not match".to_string());
            return;
        }
        self.settings.teacher_pin = self.teacher_pin_new.trim().to_string();
        self.teacher_pin_new.clear();
        self.teacher_pin_confirm.clear();
        match save_settings(&self.settings, &self.base_path) {
            Ok(_) => self.teacher_pin_status = Some("PIN updated".to_string()),
            Err(e) => self.teacher_pin_status = Some(format!("Failed to save PIN: {e}")),
        }
    }

    fn update_secret_question(&mut self) {
        if !self.teacher_unlocked {
            self.teacher_pin_status = Some("Unlock first to change secret question".to_string());
            return;
        }
        if self.teacher_secret_question_input.trim().is_empty()
            || self.teacher_secret_answer_input.trim().is_empty()
        {
            self.teacher_pin_status =
                Some("Secret question and answer cannot be empty".to_string());
            return;
        }
        self.settings.teacher_secret_question =
            self.teacher_secret_question_input.trim().to_string();
        self.settings.teacher_secret_answer = self.teacher_secret_answer_input.trim().to_string();
        match save_settings(&self.settings, &self.base_path) {
            Ok(_) => self.teacher_pin_status = Some("Secret question updated".to_string()),
            Err(e) => self.teacher_pin_status = Some(format!("Failed to save secret: {e}")),
        }
        self.teacher_secret_answer_input.clear();
    }

    fn open_teacher_dashboard(&mut self) {
        if let Some(module) = self
            .modules
            .iter()
            .find(|m| m.manifest.id == "homework_dashboard")
            .cloned()
        {
            self.open_module_tab(&module);
        }
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
        if module.manifest.id == "homework_dashboard" && !self.teacher_unlocked {
            self.teacher_pin_status =
                Some("Unlock teacher view to open the homework dashboard.".to_string());
            return;
        }
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
                ui.menu_button("Models", |ui| {
                    let models = self.available_models.clone();
                    let current_path = self.settings.model.path.clone();
                    if models.is_empty() {
                        ui.label("No models found in models/");
                    }
                    for model in models {
                        let selected = Path::new(&current_path) == model.path;
                        let file_label = model
                            .path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| n.to_string())
                            .unwrap_or_else(|| model.path.to_string_lossy().to_string());
                        let label = format!("{} ({})", model.name, file_label);
                        if ui.selectable_label(selected, label).clicked() {
                            self.select_model(&model);
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("Refresh models list").clicked() {
                        self.reload_models();
                        ui.close_menu();
                    }
                    ui.label(format!(
                        "Folder: {}",
                        self.base_path.join("models").display()
                    ));
                });
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
                let current_role = self.current_role().to_owned();
                if self.modules.is_empty() {
                    ui.label("No modules found.");
                }
                let modules = self.modules.clone();
                for module in modules {
                    if module.manifest.id == "homework_dashboard" && !self.teacher_unlocked {
                        continue;
                    }
                    if !role_allowed(&module.manifest, current_role.as_str()) {
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

            ui.menu_button("Teacher", |ui| {
                ui.label(format!(
                    "Status: {}",
                    if self.teacher_unlocked {
                        "Unlocked"
                    } else {
                        "Locked"
                    }
                ));
                ui.label(format!("Role: {}", self.current_role()));
                ui.separator();
                if !self.teacher_unlocked {
                    ui.label("Enter PIN to unlock teacher view");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.teacher_pin_input)
                            .password(true)
                            .hint_text("0000"),
                    );
                    if ui.button("Unlock").clicked() {
                        self.try_unlock_teacher();
                        ui.close_menu();
                    }
                    if !self.settings.teacher_secret_question.is_empty() {
                        ui.separator();
                        ui.label("Forgot PIN? Answer secret question:");
                        ui.label(format!("Q: {}", self.settings.teacher_secret_question));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.teacher_secret_answer_input)
                                .password(true)
                                .hint_text("Answer"),
                        );
                        if ui.button("Unlock with answer").clicked() {
                            if self.settings.teacher_secret_answer
                                == self.teacher_secret_answer_input.trim()
                            {
                                self.teacher_unlocked = true;
                                self.teacher_pin_status =
                                    Some("Unlocked via secret question".to_string());
                            } else {
                                self.teacher_pin_status =
                                    Some("Incorrect secret answer".to_string());
                            }
                            self.teacher_secret_answer_input.clear();
                            ui.close_menu();
                        }
                    }
                } else {
                    if ui.button("Open teacher dashboard").clicked() {
                        self.open_teacher_dashboard();
                        ui.close_menu();
                    }
                    if ui.button("Rescan packs + submissions").clicked() {
                        self.resync_homework();
                        self.teacher_pin_status =
                            Some("Rescanned packs and submissions.".to_string());
                    }
                    ui.separator();
                    ui.label("Class mode");
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                self.settings.teacher_mode != "class",
                                egui::Button::new("Set CLASS"),
                            )
                            .clicked()
                        {
                            self.settings.teacher_mode = "class".to_string();
                            let _ = save_settings(&self.settings, &self.base_path);
                            self.teacher_pin_status =
                                Some("Teacher mode set to CLASS.".to_string());
                        }
                        if ui
                            .add_enabled(
                                self.settings.teacher_mode != "free_time",
                                egui::Button::new("Set FREE TIME"),
                            )
                            .clicked()
                        {
                            self.settings.teacher_mode = "free_time".to_string();
                            let _ = save_settings(&self.settings, &self.base_path);
                            self.teacher_pin_status =
                                Some("Teacher mode set to FREE TIME.".to_string());
                        }
                        ui.label(format!("Current: {}", self.settings.teacher_mode));
                    });
                    ui.separator();
                    ui.label("Games");
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(!self.settings.game.enabled, egui::Button::new("Games ON"))
                            .clicked()
                        {
                            self.settings.game.enabled = true;
                            let _ = save_settings(&self.settings, &self.base_path);
                            self.teacher_pin_status = Some("Games enabled.".to_string());
                        }
                        if ui
                            .add_enabled(self.settings.game.enabled, egui::Button::new("Games OFF"))
                            .clicked()
                        {
                            self.settings.game.enabled = false;
                            let _ = save_settings(&self.settings, &self.base_path);
                            self.teacher_pin_status = Some("Games disabled.".to_string());
                        }
                        ui.label(format!(
                            "Allowed in class: {}",
                            self.settings.game.games_in_class_allowed
                        ));
                    });
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !self.settings.game.games_in_class_allowed,
                                egui::Button::new("Allow in class"),
                            )
                            .clicked()
                        {
                            self.settings.game.games_in_class_allowed = true;
                            let _ = save_settings(&self.settings, &self.base_path);
                            self.teacher_pin_status =
                                Some("Games allowed in class.".to_string());
                        }
                        if ui
                            .add_enabled(
                                self.settings.game.games_in_class_allowed,
                                egui::Button::new("Forbid in class"),
                            )
                            .clicked()
                        {
                            self.settings.game.games_in_class_allowed = false;
                            let _ = save_settings(&self.settings, &self.base_path);
                            self.teacher_pin_status =
                                Some("Games forbidden in class.".to_string());
                        }
                    });
                    ui.separator();
                    if ui.button("Export pack template").clicked() {
                        match export_pack_template(
                            &self.base_path,
                            "school",
                            &self.settings.student.class_id,
                        ) {
                            Ok(path) => {
                                self.teacher_pin_status =
                                    Some(format!("Template written to {}", path.display()));
                            }
                            Err(e) => {
                                self.teacher_pin_status =
                                    Some(format!("Failed to export template: {e}"));
                            }
                        }
                    }
                    if ui.button("Import pack file...").clicked() {
                        if let Some(file) = FileDialog::new().add_filter("json", &["json"]).pick_file() {
                            let dest_dir = self.base_path.join("homework").join("assigned");
                            let _ = fs::create_dir_all(&dest_dir);
                            let dest = dest_dir.join(
                                file.file_name()
                                    .unwrap_or_else(|| std::ffi::OsStr::new("homework_pack_import.json")),
                            );
                            match fs::copy(&file, &dest) {
                                Ok(_) => match load_pack_from_file(&dest) {
                                    Ok(pack) => {
                                        apply_pack_policy(&mut self.settings, &pack);
                                        let _ = save_settings(&self.settings, &self.base_path);
                                        self.current_pack = Some(pack);
                                        self.resync_homework();
                                        self.teacher_pin_status =
                                            Some(format!("Imported {}", dest.display()));
                                    }
                                    Err(e) => {
                                        self.teacher_pin_status =
                                            Some(format!("Copied but failed to parse pack: {e}"));
                                    }
                                },
                                Err(e) => {
                                    self.teacher_pin_status =
                                        Some(format!("Import failed: {e}"));
                                }
                            }
                        }
                    }
                    if ui.button("Show completed summary").clicked() {
                        let rows = self.submission_rows();
                        if rows.is_empty() {
                            self.teacher_pin_status =
                                Some("No completed submissions found.".to_string());
                        } else {
                            egui::Window::new("Completed submissions")
                                .collapsible(true)
                                .resizable(true)
                                .show(ui.ctx(), |ui| {
                                    ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                                        for row in &rows {
                                            let label = format!(
                                                "{} ({}) - {} ({}) - subj: {} - score: {} - {} - submitted: {}",
                                                row.assignment_title,
                                                row.assignment_id,
                                                row.student_name,
                                                row.student_id,
                                                row.subject,
                                                row.score,
                                                row.feedback,
                                                row.submitted_at
                                            );
                                            ui.label(&label);
                                            if let Some(ai_fb) = self
                                                .submissions
                                                .iter()
                                                .find(|s| {
                                                    s.assignment_id == row.assignment_id
                                                        && s.student_id == row.student_id
                                                })
                                                .and_then(|s| s.ai_feedback.clone())
                                            {
                                                ui.label(format!("AI feedback: {}", ai_fb));
                                            }
                                        }
                                    });
                                });
                            self.teacher_pin_status =
                                Some(format!("Completed submissions: {}", rows.len()));
                        }
                    }
                    if ui.button("Lock teacher view").clicked() {
                        self.lock_teacher();
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.label("Change teacher PIN");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.teacher_pin_new)
                            .password(true)
                            .hint_text("New PIN"),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.teacher_pin_confirm)
                            .password(true)
                            .hint_text("Confirm PIN"),
                    );
                    if ui.button("Update PIN").clicked() {
                        self.change_teacher_pin();
                    }
                    ui.separator();
                    ui.label("Secret question (for PIN recovery)");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.teacher_secret_question_input)
                            .hint_text("Secret question"),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.teacher_secret_answer_input)
                            .password(true)
                            .hint_text("Answer"),
                    );
                    if ui.button("Save secret").clicked() {
                        self.update_secret_question();
                    }
                }
                if let Some(msg) = &self.teacher_pin_status {
                    ui.colored_label(self.warning_color(), msg);
                }
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
        let current_model = Path::new(&self.settings.model.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&self.settings.model.path);
        ui.label(format!(
            "Active model: {} ({current_model})",
            self.settings.model.name
        ));
        ui.label(format!("Teacher mode: {}", self.settings.teacher_mode));
        ui.label(format!("Available modules: {}", self.modules.len()));
        ui.separator();
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                ui.set_min_height(ui.available_height());
                ui.label(RichText::new("Student profile").strong());
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut self.settings.student.student_name);
                    ui.label("ID");
                    ui.text_edit_singleline(&mut self.settings.student.student_id);
                    ui.label("Class");
                    ui.text_edit_singleline(&mut self.settings.student.class_id);
                    if ui.button("Save profile").clicked() {
                        let _ = save_settings(&self.settings, &self.base_path);
                    }
                });
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Rescan packs + submissions").clicked() {
                        self.resync_homework();
                    }
                    if ui
                        .add_enabled(
                            self.teacher_unlocked,
                            egui::Button::new("Export pack template"),
                        )
                        .clicked()
                    {
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
                    if ui
                        .add_enabled(
                            self.teacher_unlocked,
                            egui::Button::new("Import pack file..."),
                        )
                        .clicked()
                    {
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

        if self.teacher_unlocked {
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
                        self.draft_input.id =
                            format!("hw-{:03}", self.draft_assignments.len() + 1);
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
        } else {
            ui.separator();
            ui.colored_label(
                self.warning_color(),
                "Teacher tools are locked. Unlock via the Teacher menu to manage packs.",
            );
        }

        if let Some(pack) = self.current_pack.clone() {
            ui.separator();
            ui.label(format!(
                "Latest homework pack: {} (class {}) assignments: {}",
                pack.school_id,
                pack.class_id,
                pack.assignments.len()
            ));
            ui.horizontal(|ui| {
                ui.label("Select assignment:");
                let current = self
                    .selected_assignment_ref()
                    .map(|a| format!("{} - {}", a.id, a.title))
                    .unwrap_or_else(|| "Choose...".to_string());
                egui::ComboBox::from_id_source("home_assignment_select")
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        for a in &pack.assignments {
                            let label = format!("{} - {}", a.id, a.title);
                            if ui
                                .selectable_label(
                                    self.selected_assignment.as_ref() == Some(&a.id),
                                    label,
                                )
                                .clicked()
                            {
                                self.selected_assignment = Some(a.id.clone());
                            }
                        }
                    });
            });
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
                        ui.colored_label(self.warning_color(), "Games off");
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

        ui.add_space(12.0);

        self.render_homework_help(ui);
        });
    }

    fn render_chat(&mut self, ui: &mut egui::Ui) {
        ui.heading("Chat");
        ui.add_space(6.0);
        let log_height = ui.available_height();
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(true)
            .max_height(log_height)
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                ui.set_min_height(log_height);
                let max_width = ui.available_width() * 0.96;
                ui.set_max_width(max_width);
                for (sender, msg) in &self.chat_log {
                    let is_user = sender.eq_ignore_ascii_case("you");
                    let bubble_fill = if is_user {
                        color_from_hex(&self.theme.accent_soft)
                    } else {
                        color_from_hex(&self.theme.surface)
                    };
                    let bubble_stroke = if is_user {
                        color_from_hex(&self.theme.accent)
                    } else {
                        color_from_hex(&self.theme.border)
                    };
                    let text_color = if is_user {
                        color_from_hex(&self.theme.accent)
                    } else {
                        color_from_hex(&self.theme.text)
                    };
                    let name_color = if is_user {
                        bubble_stroke
                    } else {
                        color_from_hex(&self.theme.muted_text)
                    };

                    ui.add_space(4.0);
                    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                        ui.add_space(8.0);
                        egui::Frame::none()
                            .fill(bubble_fill)
                            .stroke(egui::Stroke {
                                width: 1.0,
                                color: bubble_stroke,
                            })
                            .rounding(egui::Rounding::same(6.0))
                            .inner_margin(egui::vec2(10.0, 8.0))
                            .show(ui, |ui| {
                                ui.set_max_width(max_width * 0.9);
                                ui.label(
                                    RichText::new(sender.clone())
                                        .strong()
                                        .color(name_color),
                                );
                                ui.add_space(4.0);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(msg).color(text_color),
                                    )
                                    .wrap(true),
                                );
                            });
                    });
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
        if !self.teacher_unlocked {
            ui.label(RichText::new("Teacher controls").strong());
            ui.colored_label(
                self.warning_color(),
                "Teacher controls are locked. Unlock via the Teacher menu.",
            );
        } else {
            ui.label(RichText::new("Teacher controls").strong());
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
            ui.checkbox(
                &mut self.settings.homework_hints_only,
                "Homework help gives hints only (no full answers)",
            );
            ui.separator();
            ui.checkbox(&mut self.settings.game.enabled, "Enable games");
            ui.checkbox(
                &mut self.settings.game.games_in_class_allowed,
                "Allow games in class",
            );
        }
        if ui.button("Save settings").clicked() {
            let _ = save_settings(&self.settings, &self.base_path);
            ui.label("Saved");
        }
    }

    fn sanitize_short(text: &str, max_lines: usize, max_len: usize) -> String {
        let mut out = String::new();
        for (i, line) in text.lines().enumerate() {
            if i >= max_lines {
                break;
            }
            if line.to_lowercase().starts_with("assistant:") || line.to_lowercase().starts_with("user:") {
                continue;
            }
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(line.trim());
        }
        if out.is_empty() {
            out = text.trim().to_string();
        }
        if out.len() > max_len {
            out.truncate(max_len);
        }
        out
    }

    fn warning_color(&self) -> egui::Color32 {
        if self.theme.name.eq_ignore_ascii_case("classic_light") {
            color_from_hex(&self.theme.accent)
        } else {
            egui::Color32::YELLOW
        }
    }

    fn render_homework_dashboard(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.heading("Homework dashboard");
                if let Some(pack) = self.current_pack.clone() {
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
                    let (class_avg, per_student_avg, per_subject_avg) =
                        aggregate_scores(&focused_entries);
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
            });
    }

    fn render_homework_assignments(&mut self, ui: &mut egui::Ui) {
        ui.heading("Homework & Revision");
        ui.label("View current assignments, questions, and quick revision tips.");

        if let Some(pack) = self.current_pack.clone() {
                ScrollArea::vertical()
                .auto_shrink([false; 2])
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    ui.separator();
                    ui.label(format!(
                        "Class: {} | Assignments: {} | School: {}",
                        pack.class_id,
                        pack.assignments.len(),
                        pack.school_id
                    ));

                    if self.selected_assignment.is_none() {
                        self.selected_assignment = pack.assignments.first().map(|a| a.id.clone());
                    }

                    ui.horizontal(|ui| {
                        ui.label("Assignment:");
                        let current_title = self
                            .selected_assignment_ref()
                            .map(|a| format!("{} - {}", a.id, a.title))
                            .unwrap_or_else(|| "Select assignment".to_string());
                        egui::ComboBox::from_id_source("module_assignment_select")
                            .selected_text(current_title)
                            .show_ui(ui, |ui| {
                                for assignment in &pack.assignments {
                                    let label = format!("{} - {}", assignment.id, assignment.title);
                                    if ui
                                        .selectable_label(
                                            self.selected_assignment.as_ref() == Some(&assignment.id),
                                            label,
                                        )
                                        .clicked()
                                    {
                                        self.selected_assignment = Some(assignment.id.clone());
                                    }
                                }
                            });
                    });

                    if let Some(assignment) = self.selected_assignment_ref() {
                        ui.separator();
                        ui.label(
                            RichText::new(&assignment.title)
                                .heading()
                                .color(color_from_hex(&self.theme.accent)),
                        );
                        ui.label(format!(
                            "{} | Year {}",
                            assignment.subject, assignment.year_level
                        ));
                        if let Some(due) = &assignment.due_at {
                            ui.label(format!("Due: {due}"));
                        } else {
                            ui.label("Due: not set");
                        }
                        ui.add_space(4.0);
                        ui.label("Instructions");
                        ui.add(
                            egui::TextEdit::multiline(&mut assignment.instructions_md.clone())
                                .interactive(false)
                                .desired_width(f32::INFINITY),
                        );
                        ui.separator();
                        ui.heading("Submit work");
                        self.render_submission_area(ui);
                    }

                    ui.separator();
                    ui.label(RichText::new("Revision tips").strong());
                    ui.label(
                        "Re-use these questions for practice. Try explaining answers in your own words, \
                         sketch graphs/gradients on paper, and ask the Chat tab for hints (not solutions).",
                    );

                    self.render_homework_help(ui);
                });
        } else {
            ui.separator();
            ui.label("No homework pack loaded. Import a pack from the Home tab to view questions.");
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

        if module.manifest.id == "homework_dashboard" && !self.teacher_unlocked {
            ui.colored_label(
                self.warning_color(),
                "Teacher view is locked. Unlock via the Teacher menu to open this dashboard.",
            );
            return;
        }

        ui.heading(&module.manifest.title);
        if let Some(desc) = &module.manifest.description {
            ui.label(desc);
        }
        ui.separator();

        match &module.manifest.entry {
            ModuleEntry::BuiltinPanel { target } => match target.as_str() {
                "homework_dashboard" => self.render_homework_dashboard(ui),
                "homework_assignments" => self.render_homework_assignments(ui),
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
                        self.warning_color(),
                        "External processes are disabled in safe mode.",
                    );
                }
            }
        }
    }

    fn selected_assignment_ref(&self) -> Option<&HomeworkAssignment> {
        let pack = self.current_pack.as_ref()?;
        if let Some(id) = &self.selected_assignment {
            if let Some(found) = pack.assignments.iter().find(|a| &a.id == id) {
                return Some(found);
            }
        }
        pack.assignments.first()
    }

    fn render_homework_help(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.heading("Ask for hints");
        let assignment_opt = self.selected_assignment_ref().cloned();
        if assignment_opt.is_none() {
            ui.label("Select an assignment to ask for hints.");
        }
        let assignment = match &assignment_opt {
            Some(a) => a.clone(),
            None => {
                ui.add_enabled(false, egui::Button::new("Get hints"));
                return;
            }
        };

        ui.label(format!(
            "Context: {} ({}) | Subject: {} | Year {}",
            assignment.title, assignment.id, assignment.subject, assignment.year_level
        ));
        ui.add(
            egui::TextEdit::multiline(&mut self.homework_help_question)
                .hint_text(
                    "Ask for a hint or explanation. We'll keep it short and avoid giving full answers.",
                )
                .desired_width(f32::INFINITY),
        );
        let get_hints = ui.add_enabled(assignment_opt.is_some(), egui::Button::new("Get hints"));
        if get_hints.clicked() {
            let question = self.homework_help_question.trim().to_string();
            if question.is_empty() {
                self.homework_help_status = Some("Type a question first.".to_string());
            } else {
                self.homework_help_status = Some("Generating hints...".to_string());
                let prompt = format!(
                    "{capsule}\nAssignment: {id} - {title}\nSubject: {subject}\nYear: {year}\nDue: {due}\nInstructions:\n{instr}\nStudent question: {q}\nRespond with one short hint (guiding question, steps, or reminder). Never provide the full answer.",
                    capsule = HINT_CAPSULE,
                    id = assignment.id,
                    title = assignment.title,
                    subject = assignment.subject,
                    year = assignment.year_level,
                    due = assignment.due_at.clone().unwrap_or_else(|| "not set".to_string()),
                    instr = assignment.instructions_md,
                    q = question
                );
                let result = panic::catch_unwind({
                    let settings = self.settings.clone();
                    move || {
                        let raw = generate_answer(&settings, &prompt);
                        raw
                    }
                });
                match result {
                    Ok(text) => {
                        self.homework_help_response =
                            Some(Self::sanitize_short(&text, 4, 400));
                        self.homework_help_status = Some("Hints ready.".to_string());
                    }
                    Err(_) => {
                        self.homework_help_response = None;
                        self.homework_help_status =
                            Some("Sorry, something went wrong while generating hints.".to_string());
                    }
                }
            }
        }
        if let Some(status) = &self.homework_help_status {
            ui.label(status);
        }
        if let Some(resp) = &self.homework_help_response {
            ui.add_space(4.0);
            ui.label(RichText::new(resp).color(color_from_hex(&self.theme.text)));
        }
    }

    fn render_submission_area(&mut self, ui: &mut egui::Ui) {
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
        // Show a placeholder before generation to avoid disappearing messages
        self.chat_log
            .push(("Chatty".to_string(), "...".to_string()));

        let result = panic::catch_unwind({
            let settings = self.settings.clone();
            let question = user_msg.clone();
            move || {
                let prompt = format!(
                    "{capsule}\nUser request: {q}\nRespond with one short, clear answer.",
                    capsule = CHAT_CAPSULE,
                    q = question
                );
                generate_answer(&settings, &prompt)
            }
        });

        if let Some(last) = self.chat_log.last_mut() {
            last.1 = match result {
                Ok(filtered) => Self::sanitize_short(&filtered, 4, 400),
                Err(_) => "Sorry, I ran into an error while answering.".to_string(),
            };
        }
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

fn color_from_hex(hex: &str) -> egui::Color32 {
    let h = hex.trim_start_matches('#');
    if h.len() == 6 {
        if let Ok(rgb) = u32::from_str_radix(h, 16) {
            let r = ((rgb >> 16) & 0xFF) as u8;
            let g = ((rgb >> 8) & 0xFF) as u8;
            let b = (rgb & 0xFF) as u8;
            return egui::Color32::from_rgb(r, g, b);
        }
    } else if h.len() == 8 {
        if let Ok(rgba) = u32::from_str_radix(h, 16) {
            let r = ((rgba >> 24) & 0xFF) as u8;
            let g = ((rgba >> 16) & 0xFF) as u8;
            let b = ((rgba >> 8) & 0xFF) as u8;
            let a = (rgba & 0xFF) as u8;
            return egui::Color32::from_rgba_premultiplied(r, g, b, a);
        }
    }
    egui::Color32::GRAY
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

