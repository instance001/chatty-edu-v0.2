use clap::{Parser, ValueEnum};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

mod chat;
mod gui;
mod homework;
mod homework_pack;
mod local_model;
mod modules;
mod settings;
mod theme;

use chat::{generate_answer, janet_filter};
use homework_pack::{
    apply_pack_policy, create_pack, create_pack_multi, export_pack_template, find_latest_pack,
    load_pack_from_file, load_submission_summaries, save_submission_with_answers,
    HomeworkAssignment,
};
use settings::{
    default_base_path, ensure_base_folders, load_or_init_settings, save_settings, Settings,
};

#[derive(Parser, Debug)]
#[command(
    name = "chatty-edu",
    version,
    about = "Chatty-EDU v0.2 shell (local-first, offline)"
)]
struct CliArgs {
    /// Choose GUI (default) or CLI mode
    #[arg(long, value_enum, default_value = "gui")]
    mode: RunMode,
    /// Override data base path (defaults to ./data next to the exe)
    #[arg(long)]
    base_path: Option<PathBuf>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum RunMode {
    Gui,
    Cli,
}

fn main() {
    let args = CliArgs::parse();
    let base_path = args.base_path.unwrap_or_else(default_base_path);

    if let Err(e) = ensure_base_folders(&base_path) {
        eprintln!(
            "Failed to create base folders at {}: {}",
            base_path.display(),
            e
        );
        return;
    }

    let mut settings = match load_or_init_settings(&base_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to load settings: {}", e);
            return;
        }
    };

    println!("Using data path: {}", base_path.display());

    // Apply latest homework pack policy (e.g., games allowed/blocked) if present.
    if let Ok(Some((_pack_path, pack))) = find_latest_pack(&base_path) {
        apply_pack_policy(&mut settings, &pack);
    }

    settings.base_path = base_path.to_string_lossy().to_string();
    settings.mode = match args.mode {
        RunMode::Gui => "gui".to_string(),
        RunMode::Cli => "cli".to_string(),
    };

    match args.mode {
        RunMode::Gui => {
            if let Err(e) = gui::launch_gui(base_path.clone(), settings.clone()) {
                eprintln!("Failed to start GUI: {}", e);
            }
        }
        RunMode::Cli => {
            run_cli(&mut settings, &base_path);
        }
    }

    if let Err(e) = save_settings(&settings, &base_path) {
        eprintln!("Could not save settings: {}", e);
    }
}

fn run_cli(settings: &mut Settings, base_path: &Path) {
    println!("Chatty-EDU v0.2 CLI starting up");
    println!("Base path: {}", base_path.display());
    println!("Mode: {}", settings.mode);
    println!("Type 'exit' to quit, 'teacher' for teacher console, 'play' to try game mode.\n");

    loop {
        println!(
            "[Mode: {} | TeacherMode: {}]",
            settings.mode, settings.teacher_mode
        );
        print!("You (or command): ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("Error reading input. Exiting.");
            break;
        }

        let input = input.trim();
        if input.eq_ignore_ascii_case("exit") {
            println!("Goodbye");
            break;
        }

        if input.eq_ignore_ascii_case("teacher") {
            teacher_console(settings, base_path);
            continue;
        }

        if let Some(rest) = input.strip_prefix("submit ") {
            let assignment_id = rest.trim();
            if assignment_id.is_empty() {
                println!("Usage: submit <assignment_id>");
            } else {
                let answers = prompt("Answer text", "My work goes here").unwrap_or_default();
                let attachments_input =
                    prompt("Attachment paths (comma-separated, optional)", "").unwrap_or_default();
                let attachments: Vec<String> = attachments_input
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                match save_submission_with_answers(
                    base_path,
                    settings,
                    assignment_id,
                    &answers,
                    &attachments,
                ) {
                    Ok(path) => println!("Wrote submission to {}", path.display()),
                    Err(e) => println!("Failed to write submission: {}", e),
                }
            }
            continue;
        }

        if input.to_lowercase().starts_with("play") {
            handle_play_request(settings);
            continue;
        }

        if input.is_empty() {
            continue;
        }

        let raw_answer = generate_answer(settings, input);
        let safe_answer = janet_filter(&settings.janet, &raw_answer, input);

        println!("Chatty: {safe_answer}\n");
    }
}

fn handle_play_request(settings: &Settings) {
    if !settings.game.enabled {
        println!("\n[Play] Games are currently DISABLED in settings.\n");
        return;
    }

    if settings.teacher_mode == "class" && !settings.game.games_in_class_allowed {
        println!("\n[Play] Games are not allowed in CLASS mode.\n");
        return;
    }

    println!("\n[Play] Game mode is not implemented yet, but the hook is working.\n");
}

fn teacher_console(settings: &mut Settings, base_path: &Path) {
    println!("\nEnter teacher PIN (default 0000):");
    println!("Type 'forgot' to answer the secret question.");
    print!("PIN: ");
    io::stdout().flush().unwrap();

    let mut pin_input = String::new();
    if let Err(e) = io::stdin().read_line(&mut pin_input) {
        println!("Failed to read PIN: {}", e);
        return;
    }

    let pin_input = pin_input.trim();
    if pin_input.eq_ignore_ascii_case("forgot") {
        println!("Secret question: {}", settings.teacher_secret_question);
        print!("Answer: ");
        io::stdout().flush().unwrap();
        let mut answer = String::new();
        if let Err(e) = io::stdin().read_line(&mut answer) {
            println!("Failed to read answer: {}", e);
            return;
        }
        if answer.trim() != settings.teacher_secret_answer {
            println!("Incorrect answer.\n");
            return;
        }
        println!("Unlocked with secret question.\n");
    } else if settings.teacher_pin != pin_input {
        println!("Incorrect PIN.\n");
        return;
    }

    println!("\nTeacher console\n");

    loop {
        println!("Current teacher mode: {}", settings.teacher_mode);
        println!("Games enabled: {}", settings.game.enabled);
        println!(
            "Games allowed in class: {}",
            settings.game.games_in_class_allowed
        );
        println!("Base path: {}", base_path.display());
        println!(
            "Homework (assigned): {}",
            base_path.join("homework").join("assigned").display()
        );
        println!(
            "Homework (completed): {}",
            base_path.join("homework").join("completed").display()
        );
        println!("Commands:");
        println!("  mode class");
        println!("  mode free");
        println!("  games on");
        println!("  games off");
        println!("  allow_games_in_class");
        println!("  forbid_games_in_class");
        println!("  show_completed    (show table of completed homework)");
        println!("  homework table    (alias for show_completed)");
        println!("  export_pack_template  (writes a homework_pack template to assigned/)");
        println!("  create_pack           (interactive pack builder, single assignment)");
        println!("  create_pack_multi     (interactive pack builder, multi assignment)");
        println!("  import_submissions    (summarize submission_*.json in completed/)");
        println!(
            "  import_pack <path>    (copy a pack file into homework/assigned/ and apply policy)"
        );
        println!("  set_pin               (change teacher PIN; confirm twice)");
        println!("  set_secret            (change secret question + answer)");
        println!("  back");

        print!("teacher> ");
        io::stdout().flush().ok();

        let mut input = String::new();
        if let Err(e) = io::stdin().read_line(&mut input) {
            println!("Input error ({}), exiting teacher console.", e);
            break;
        }

        let cmd = input.trim();

        match cmd {
            "mode class" => {
                settings.teacher_mode = "class".to_string();
                println!("Teacher mode set to CLASS.");
            }
            "mode free" => {
                settings.teacher_mode = "free_time".to_string();
                println!("Teacher mode set to FREE TIME.");
            }
            "games on" => {
                settings.game.enabled = true;
                println!("Games ENABLED.");
            }
            "games off" => {
                settings.game.enabled = false;
                println!("Games DISABLED.");
            }
            "allow_games_in_class" => {
                settings.game.games_in_class_allowed = true;
                println!("Games allowed in CLASS mode.");
            }
            "forbid_games_in_class" => {
                settings.game.games_in_class_allowed = false;
                println!("Games forbidden in CLASS mode.");
            }
            "show_completed" | "homework table" => {
                homework::show_homework_dashboard(base_path);
            }
            "export_pack_template" => {
                match export_pack_template(base_path, "school", &settings.student.class_id) {
                    Ok(path) => println!("Pack template written to {}", path.display()),
                    Err(e) => println!("Failed to write template: {}", e),
                }
            }
            "create_pack" => match create_pack_interactive(base_path) {
                Ok(path) => println!("Pack written to {}", path.display()),
                Err(e) => println!("Failed to write pack: {}", e),
            },
            "create_pack_multi" => match create_pack_multi_interactive(base_path) {
                Ok(path) => println!("Pack written to {}", path.display()),
                Err(e) => println!("Failed to write pack: {}", e),
            },
            "import_submissions" => match load_submission_summaries(base_path) {
                Ok(list) => {
                    if list.is_empty() {
                        println!("No submission_*.json files found in completed/.");
                    } else {
                        println!("Submissions:");
                        for s in list {
                            let score = s
                                .score
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "-".to_string());
                            println!(
                                "  {} by {} ({}) score: {}",
                                s.assignment_id, s.student_name, s.student_id, score
                            );
                        }
                    }
                }
                Err(e) => println!("Failed to read submissions: {}", e),
            },
            _ if cmd.starts_with("import_pack ") => {
                let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
                if parts.len() < 2 {
                    println!("Usage: import_pack <path_to_pack.json>");
                } else {
                    let src = PathBuf::from(parts[1].trim());
                    if !src.exists() {
                        println!("File not found: {}", src.display());
                    } else {
                        let dest_dir = base_path.join("homework").join("assigned");
                        if let Err(e) = std::fs::create_dir_all(&dest_dir) {
                            println!("Failed to create assigned dir: {}", e);
                            continue;
                        }
                        let dest =
                            dest_dir.join(src.file_name().unwrap_or_else(|| {
                                std::ffi::OsStr::new("homework_pack_import.json")
                            }));
                        match std::fs::copy(&src, &dest) {
                            Ok(_) => match load_pack_from_file(&dest) {
                                Ok(pack) => {
                                    apply_pack_policy(settings, &pack);
                                    if let Err(e) = save_settings(settings, base_path) {
                                        println!(
                                            "Imported pack but failed to save settings: {}",
                                            e
                                        );
                                    } else {
                                        println!(
                                            "Imported pack to {} and applied policy.",
                                            dest.display()
                                        );
                                    }
                                }
                                Err(e) => println!("Copied but failed to parse pack: {}", e),
                            },
                            Err(e) => println!("Copy failed: {}", e),
                        }
                    }
                }
            }
            "back" => {
                if let Err(e) = save_settings(settings, base_path) {
                    println!("Failed to save settings: {}", e);
                }
                println!("Exiting teacher console.\n");
                break;
            }
            "set_pin" => {
                let new_pin = match prompt("New PIN", "") {
                    Ok(v) => v,
                    Err(e) => {
                        println!("Failed to read PIN: {}", e);
                        continue;
                    }
                };
                let confirm_pin = match prompt("Confirm PIN", "") {
                    Ok(v) => v,
                    Err(e) => {
                        println!("Failed to read PIN confirmation: {}", e);
                        continue;
                    }
                };
                if new_pin.trim().is_empty() {
                    println!("PIN cannot be empty.");
                    continue;
                }
                if new_pin != confirm_pin {
                    println!("PINs did not match. PIN unchanged.");
                    continue;
                }
                settings.teacher_pin = new_pin;
                if let Err(e) = save_settings(settings, base_path) {
                    println!("PIN updated but failed to save settings: {}", e);
                } else {
                    println!("Teacher PIN updated.");
                }
            }
            "set_secret" => {
                let question = match prompt("New secret question", "") {
                    Ok(v) => v,
                    Err(e) => {
                        println!("Failed to read question: {}", e);
                        continue;
                    }
                };
                let answer = match prompt("New secret answer", "") {
                    Ok(v) => v,
                    Err(e) => {
                        println!("Failed to read answer: {}", e);
                        continue;
                    }
                };
                if question.trim().is_empty() || answer.trim().is_empty() {
                    println!("Question and answer cannot be empty.");
                    continue;
                }
                settings.teacher_secret_question = question.trim().to_string();
                settings.teacher_secret_answer = answer.trim().to_string();
                if let Err(e) = save_settings(settings, base_path) {
                    println!("Secret updated but failed to save settings: {}", e);
                } else {
                    println!("Secret question/answer updated.");
                }
            }
            _ => println!("Unknown command."),
        }
    }
}

fn create_pack_interactive(base_path: &Path) -> io::Result<PathBuf> {
    println!("Creating homework pack (single assignment). Leave blank for defaults.");
    let school_id = prompt("School ID", "school")?;
    let class_id = prompt("Class ID", "class")?;
    let assignment_id = prompt("Assignment ID", "hw-001")?;
    let title = prompt("Title", "Homework")?;
    let subject = prompt("Subject", "General")?;
    let year_level = prompt("Year level", "7")?;
    let due_at = prompt("Due at (ISO8601, optional)", "")?;
    let allow_games = prompt("Allow games? (y/n)", "n")?
        .to_lowercase()
        .starts_with('y');
    let allow_ai_premark = prompt("Allow AI premark? (y/n)", "y")?
        .to_lowercase()
        .starts_with('y');
    let max_score = prompt("Max score (int, optional)", "")?;
    let instructions = prompt("Instructions (one line)", "Add details here.")?;

    let assignment = HomeworkAssignment {
        id: assignment_id,
        title,
        subject,
        year_level,
        due_at: if due_at.is_empty() {
            None
        } else {
            Some(due_at)
        },
        instructions_md: instructions,
        attachments: vec![],
        allow_games,
        allow_ai_premark,
        max_score: if max_score.is_empty() {
            None
        } else {
            max_score.parse().ok()
        },
    };

    create_pack(base_path, &school_id, &class_id, assignment)
}

fn prompt(field: &str, default_val: &str) -> io::Result<String> {
    print!("{} [{}]: ", field, default_val);
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        Ok(default_val.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn create_pack_multi_interactive(base_path: &Path) -> io::Result<PathBuf> {
    println!("Creating homework pack (multiple assignments). Leave blank for defaults. Enter 'done' for Assignment ID to finish.");
    let school_id = prompt("School ID", "school")?;
    let class_id = prompt("Class ID", "class")?;

    let mut assignments: Vec<HomeworkAssignment> = Vec::new();
    loop {
        let assignment_id = prompt("Assignment ID", "")?;
        if assignment_id.trim().is_empty() || assignment_id.trim().eq_ignore_ascii_case("done") {
            break;
        }
        let title = prompt("Title", "Homework")?;
        let subject = prompt("Subject", "General")?;
        let year_level = prompt("Year level", "7")?;
        let due_at = prompt("Due at (ISO8601, optional)", "")?;
        let allow_games = prompt("Allow games? (y/n)", "n")?
            .to_lowercase()
            .starts_with('y');
        let allow_ai_premark = prompt("Allow AI premark? (y/n)", "y")?
            .to_lowercase()
            .starts_with('y');
        let max_score = prompt("Max score (int, optional)", "")?;
        let instructions = prompt("Instructions (one line)", "Add details here.")?;

        let assignment = HomeworkAssignment {
            id: assignment_id,
            title,
            subject,
            year_level,
            due_at: if due_at.is_empty() {
                None
            } else {
                Some(due_at)
            },
            instructions_md: instructions,
            attachments: vec![],
            allow_games,
            allow_ai_premark,
            max_score: if max_score.is_empty() {
                None
            } else {
                max_score.parse().ok()
            },
        };
        assignments.push(assignment);
    }

    if assignments.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "No assignments added.",
        ));
    }

    create_pack_multi(base_path, &school_id, &class_id, assignments)
}
