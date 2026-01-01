use std::fs;
use std::path::Path;

use crate::gui::models::{AppConfig, PolicyConfig, ThemeConfig, ThemeColors};

fn load_json_or_default<T: serde::de::DeserializeOwned + Default>(path: &Path) -> T {
    match fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => T::default(),
    }
}

pub fn load_app_config(base: &Path) -> AppConfig {
    load_json_or_default(&base.join("config").join("app.json"))
}

pub fn load_policy_config(base: &Path) -> PolicyConfig {
    load_json_or_default(&base.join("config").join("policy.json"))
}

pub fn load_theme(base: &Path, theme_filename: &str) -> ThemeConfig {
    let p = base.join("themes").join(theme_filename);
    match fs::read_to_string(&p) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| fallback_theme()),
        Err(_) => fallback_theme(),
    }
}

fn fallback_theme() -> ThemeConfig {
    ThemeConfig {
        name: "Fallback".to_string(),
        font_scale: Some(1.0),
        rounding: Some(8.0),
        spacing: Some(8.0),
        colors: ThemeColors {
            bg: "#FFFFFF".into(),
            panel: "#F4F4F4".into(),
            text: "#111111".into(),
            muted_text: "#444444".into(),
            accent: "#2A6DF4".into(),
            danger: "#CC3333".into(),
        },
    }
}
