use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
mod homework;

const APP_FOLDER_NAME: &str = "Chatty-EDU";

#[derive(Serialize, Deserialize, Debug)]
struct JanetConfig {
    enabled: bool,
    block_swears: bool,
    block_mature_topics: bool,
    fallback_message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ModelConfig {
    name: String,
    path: String,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct VoiceConfig {
    enabled: bool,
    engine: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GameConfig {
    enabled: bool,
    games_in_class_allowed: bool,
    available_games: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Settings {
    version: String,
    base_path: String,
    mode: String,
    default_year_level: String,
    teacher_mode: String,
    janet: JanetConfig,
    model: ModelConfig,
    voice: VoiceConfig,
    game: GameConfig,
}

fn main() {
    println!("🍎 Chatty-EDU v0.1 — base spine starting up…");

    let base_path = get_base_path();
    ensure_base_folders(&base_path).expect("Failed to create base folders");
    let mut settings = load_or_init_settings(&base_path).expect("Failed to load settings");

    println!("Base path: {}", settings.base_path);
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
            println!("Goodbye 👋");
            break;
        }

        if input.eq_ignore_ascii_case("teacher") {
            teacher_console(&mut settings, &base_path);
            continue;
        }

        if input.to_lowercase().starts_with("play") {
            handle_play_request(&settings);
            continue;
        }

        if input.is_empty() {
            continue;
        }

        let raw_answer = generate_answer_stub(input);
        let safe_answer = janet_filter(&settings.janet, &raw_answer, input);

        println!("Chatty: {safe_answer}\n");
    }
}

fn get_base_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(format!("C:\\{}", APP_FOLDER_NAME))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(APP_FOLDER_NAME)
    }
}

fn ensure_base_folders(base: &Path) -> io::Result<()> {
    let dirs = [
        base.to_path_buf(),
        base.join("homework"),
        base.join("homework").join("assigned"),
        base.join("homework").join("completed"),
        base.join("revision"),
        base.join("modules"),
        base.join("logs"),
        base.join("config"),
        base.join("runtime"),
    ];

    for d in dirs {
        if !d.exists() {
            fs::create_dir_all(&d)?;
            println!("Created folder: {}", d.display());
        }
    }

    Ok(())
}

fn load_or_init_settings(base: &Path) -> io::Result<Settings> {
    let config_path = base.join("config").join("settings.json");

    if config_path.exists() {
        let contents = fs::read_to_string(&config_path)?;
        let settings: Settings = serde_json::from_str(&contents)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("JSON parse error: {e}")))?;
        return Ok(settings);
    }

    let settings = Settings {
        version: "0.1.0".to_string(),
        base_path: base.to_string_lossy().to_string(),
        mode: "cli".to_string(),
        default_year_level: "year_3".to_string(),
        teacher_mode: "class".to_string(),
        janet: JanetConfig {
            enabled: true,
            block_swears: true,
            block_mature_topics: true,
            fallback_message: "Let's ask a teacher or parent about that one.".to_string(),
        },
        model: ModelConfig {
            name: "phi-mini-placeholder".to_string(),
            path: base
                .join("runtime")
                .join("model.gguf")
                .to_string_lossy()
                .to_string(),
            max_tokens: 256,
        },
        voice: VoiceConfig {
            enabled: false,
            engine: "os_tts".to_string(),
        },
        game: GameConfig {
            enabled: true,
            games_in_class_allowed: false,
            available_games: vec!["chattybox".to_string(), "chattyclysm".to_string()],
        },
    };

    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("JSON encode error: {e}")))?;
    fs::write(&config_path, json)?;
    println!("Created default settings at {}", config_path.display());

    Ok(settings)
}

fn save_settings(settings: &Settings, base: &Path) -> io::Result<()> {
    let config_path = base.join("config").join("settings.json");
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("JSON encode error: {e}")))?;
    fs::write(&config_path, json)?;
    Ok(())
}

fn generate_answer_stub(user_input: &str) -> String {
    format!(
        "This is a placeholder answer for: \"{}\".\nOnce the model is wired, I'll explain this properly.",
        user_input
    )
}

fn janet_filter(janet: &JanetConfig, answer: &str, user_input: &str) -> String {
    if !janet.enabled {
        return answer.to_string();
    }

    let banned_swears = ["fuck", "shit", "cunt", "bitch", "bastard"];
    let banned_mature = ["sex", "porn", "drugs", "suicide", "kill", "terrorist"];

    let lower_in = user_input.to_lowercase();
    let lower_ans = answer.to_lowercase();

    let contains_swear = janet.block_swears
        && banned_swears
            .iter()
            .any(|w| lower_in.contains(w) || lower_ans.contains(w));

    let contains_mature = janet.block_mature_topics
        && banned_mature
            .iter()
            .any(|w| lower_in.contains(w) || lower_ans.contains(w));

    if contains_swear || contains_mature {
        janet.fallback_message.clone()
    } else {
        answer.to_string()
    }
}
fn handle_play_request(settings: &Settings) {
    // For now, just respect the game settings and print a message.
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
    use std::io::Write;

    println!("\n🔐 Enter teacher PIN (stubbed for now, no check):");
    print!("PIN: ");
    io::stdout().flush().unwrap();

    let mut pin_input = String::new();
    if let Err(e) = io::stdin().read_line(&mut pin_input) {
        println!("Failed to read PIN: {}", e);
        return;
    }

    println!("\n👩‍🏫 Teacher console\n");

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
            "back" => {
                if let Err(e) = save_settings(settings, base_path) {
                    println!("Failed to save settings: {}", e);
                }
                println!("Exiting teacher console.\n");
                break;
            }
            _ => println!("Unknown command."),
        }
    }
}
