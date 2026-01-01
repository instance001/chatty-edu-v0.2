use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const APP_FOLDER_NAME: &str = "Chatty-EDU";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JanetConfig {
    pub enabled: bool,
    pub block_swears: bool,
    pub block_mature_topics: bool,
    pub fallback_message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelConfig {
    pub name: String,
    pub path: String,
    pub max_tokens: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VoiceConfig {
    pub enabled: bool,
    pub engine: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameConfig {
    pub enabled: bool,
    pub games_in_class_allowed: bool,
    pub available_games: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UiSettings {
    #[serde(default)]
    pub last_theme: Option<String>,
    #[serde(default)]
    pub window_size: Option<(f32, f32)>,
    #[serde(default)]
    pub restore_tabs: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    pub version: String,
    pub base_path: String,
    pub mode: String,
    pub default_year_level: String,
    pub teacher_mode: String,
    #[serde(default)]
    pub student: StudentProfile,
    pub janet: JanetConfig,
    pub model: ModelConfig,
    pub voice: VoiceConfig,
    pub game: GameConfig,
    #[serde(default)]
    pub ui: UiSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct StudentProfile {
    #[serde(default)]
    pub student_id: String,
    #[serde(default)]
    pub student_name: String,
    #[serde(default)]
    pub class_id: String,
}

pub fn default_base_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    if let Some(dir) = exe_dir {
        return dir.join("data");
    }

    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_FOLDER_NAME)
}

pub fn ensure_base_folders(base: &Path) -> io::Result<()> {
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
        base.join("themes"),
    ];

    for d in dirs {
        if !d.exists() {
            fs::create_dir_all(&d)?;
        }
    }

    Ok(())
}

pub fn settings_path(base: &Path) -> PathBuf {
    base.join("config").join("settings.json")
}

pub fn load_or_init_settings(base: &Path) -> io::Result<Settings> {
    let config_path = settings_path(base);

    if config_path.exists() {
        let contents = fs::read_to_string(&config_path)?;
        let mut settings: Settings = serde_json::from_str(&contents)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("JSON parse error: {e}")))?;

        // Ensure base_path stays in sync with the current base
        if settings.base_path != base.to_string_lossy() {
            settings.base_path = base.to_string_lossy().to_string();
        }
        return Ok(settings);
    }

    let settings = Settings {
        version: "0.2.0".to_string(),
        base_path: base.to_string_lossy().to_string(),
        mode: "gui".to_string(),
        default_year_level: "year_3".to_string(),
        teacher_mode: "class".to_string(),
        student: StudentProfile {
            student_id: "student-id-placeholder".to_string(),
            student_name: "Student Name".to_string(),
            class_id: "class-placeholder".to_string(),
        },
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
        ui: UiSettings::default(),
    };

    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("JSON encode error: {e}")))?;
    fs::write(&config_path, json)?;

    Ok(settings)
}

pub fn save_settings(settings: &Settings, base: &Path) -> io::Result<()> {
    let config_path = settings_path(base);
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("JSON encode error: {e}")))?;
    fs::write(&config_path, json)?;
    Ok(())
}
