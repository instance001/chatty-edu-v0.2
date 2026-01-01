use eframe::egui::{self, Color32, Context, Rounding};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: String,
    pub surface: String,
    pub panel: String,
    pub text: String,
    pub muted_text: String,
    pub accent: String,
    pub accent_soft: String,
    pub border: String,
    pub radius: f32,
    pub shadow: f32,
    pub font_size_base: f32,
}

pub fn themes_dir(base: &Path) -> PathBuf {
    base.join("themes")
}

pub fn theme_file(base: &Path) -> PathBuf {
    themes_dir(base).join("theme.json")
}

pub fn presets_file(base: &Path) -> PathBuf {
    themes_dir(base).join("presets.json")
}

pub fn ensure_theme_files(base: &Path) -> io::Result<()> {
    let dir = themes_dir(base);
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }

    let presets_path = presets_file(base);
    if !presets_path.exists() {
        let presets = default_presets();
        let json = serde_json::to_string_pretty(&presets)?;
        fs::write(&presets_path, json)?;
    }

    let active_path = theme_file(base);
    if !active_path.exists() {
        let default_theme = default_presets()
            .into_iter()
            .find(|t| t.name == "classic_light")
            .unwrap_or_else(|| default_presets()[0].clone());
        let json = serde_json::to_string_pretty(&default_theme)?;
        fs::write(&active_path, json)?;
    }

    Ok(())
}

pub fn load_presets(base: &Path) -> Vec<ThemeConfig> {
    let presets_path = presets_file(base);
    if let Ok(contents) = fs::read_to_string(&presets_path) {
        if let Ok(list) = serde_json::from_str::<Vec<ThemeConfig>>(&contents) {
            return list;
        }
    }
    default_presets()
}

pub fn load_theme(base: &Path, preferred: Option<&str>) -> ThemeConfig {
    let presets = load_presets(base);
    if let Some(name) = preferred {
        if let Some(found) = presets.iter().find(|p| p.name == name) {
            return found.clone();
        }
    }

    let active_path = theme_file(base);
    if let Ok(contents) = fs::read_to_string(&active_path) {
        if let Ok(theme) = serde_json::from_str::<ThemeConfig>(&contents) {
            return theme;
        }
    }

    presets
        .into_iter()
        .find(|t| t.name == "classic_light")
        .unwrap_or_else(|| default_presets()[0].clone())
}

pub fn save_theme(base: &Path, theme: &ThemeConfig) -> io::Result<()> {
    let json = serde_json::to_string_pretty(theme)?;
    fs::write(theme_file(base), json)?;
    Ok(())
}

pub fn apply_theme(theme: &ThemeConfig, ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    let mut visuals = if is_dark(&theme) {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.panel_fill = parse_color(&theme.panel);
    visuals.widgets.noninteractive.bg_fill = parse_color(&theme.surface);
    visuals.widgets.noninteractive.fg_stroke.color = parse_color(&theme.text);
    visuals.widgets.inactive.bg_fill = parse_color(&theme.surface);
    visuals.widgets.inactive.fg_stroke.color = parse_color(&theme.text);
    visuals.widgets.inactive.bg_stroke.color = parse_color(&theme.border);

    visuals.widgets.hovered.bg_fill = parse_color(&theme.accent_soft);
    visuals.widgets.hovered.bg_stroke.color = parse_color(&theme.accent);
    visuals.widgets.hovered.fg_stroke.color = parse_color(&theme.text);

    visuals.widgets.active.bg_fill = parse_color(&theme.accent_soft);
    visuals.widgets.active.bg_stroke.color = parse_color(&theme.accent);
    visuals.widgets.active.fg_stroke.color = parse_color(&theme.text);

    visuals.window_rounding = Rounding::same(theme.radius);
    visuals.widgets.noninteractive.rounding = Rounding::same(theme.radius);
    visuals.widgets.inactive.rounding = Rounding::same(theme.radius);
    visuals.widgets.hovered.rounding = Rounding::same(theme.radius);
    visuals.widgets.active.rounding = Rounding::same(theme.radius);

    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 6.0),
        blur: theme.shadow,
        spread: 0.0,
        color: Color32::from_black_alpha(40),
    };
    visuals.popup_shadow = visuals.window_shadow;

    style.text_styles = [
        (
            egui::TextStyle::Small,
            egui::FontId::proportional(theme.font_size_base - 2.0),
        ),
        (
            egui::TextStyle::Body,
            egui::FontId::proportional(theme.font_size_base),
        ),
        (
            egui::TextStyle::Button,
            egui::FontId::proportional(theme.font_size_base),
        ),
        (
            egui::TextStyle::Heading,
            egui::FontId::proportional(theme.font_size_base + 6.0),
        ),
        (
            egui::TextStyle::Monospace,
            egui::FontId::monospace(theme.font_size_base - 1.0),
        ),
    ]
    .into();
    style.visuals = visuals;
    ctx.set_style(style);
}

fn is_dark(theme: &ThemeConfig) -> bool {
    let bg = parse_color(&theme.panel);
    // Simple luminance check; lower means darker.
    let luminance = 0.2126 * (bg.r() as f32) + 0.7152 * (bg.g() as f32) + 0.0722 * (bg.b() as f32);
    luminance < 128.0
}

fn parse_color(hex: &str) -> Color32 {
    let h = hex.trim_start_matches('#');
    if h.len() == 6 {
        if let Ok(rgb) = u32::from_str_radix(h, 16) {
            let r = ((rgb >> 16) & 0xFF) as u8;
            let g = ((rgb >> 8) & 0xFF) as u8;
            let b = (rgb & 0xFF) as u8;
            return Color32::from_rgb(r, g, b);
        }
    } else if h.len() == 8 {
        if let Ok(rgba) = u32::from_str_radix(h, 16) {
            let r = ((rgba >> 24) & 0xFF) as u8;
            let g = ((rgba >> 16) & 0xFF) as u8;
            let b = ((rgba >> 8) & 0xFF) as u8;
            let a = (rgba & 0xFF) as u8;
            return Color32::from_rgba_premultiplied(r, g, b, a);
        }
    }
    Color32::LIGHT_GRAY
}

pub fn default_presets() -> Vec<ThemeConfig> {
    vec![
        ThemeConfig {
            name: "classic_light".to_string(),
            surface: "#f5f6fa".to_string(),
            panel: "#ffffff".to_string(),
            text: "#1f2933".to_string(),
            muted_text: "#637588".to_string(),
            accent: "#2b78e4".to_string(),
            accent_soft: "#dfe9ff".to_string(),
            border: "#d0d5dc".to_string(),
            radius: 6.0,
            shadow: 8.0,
            font_size_base: 16.0,
        },
        ThemeConfig {
            name: "chalkboard_dark".to_string(),
            surface: "#1f2a33".to_string(),
            panel: "#15202b".to_string(),
            text: "#e5f0ff".to_string(),
            muted_text: "#9bb2c7".to_string(),
            accent: "#4caf50".to_string(),
            accent_soft: "#23402a".to_string(),
            border: "#2e3c48".to_string(),
            radius: 6.0,
            shadow: 10.0,
            font_size_base: 16.0,
        },
        ThemeConfig {
            name: "high_contrast".to_string(),
            surface: "#000000".to_string(),
            panel: "#0d0d0d".to_string(),
            text: "#ffffff".to_string(),
            muted_text: "#c7c7c7".to_string(),
            accent: "#ffcc00".to_string(),
            accent_soft: "#4d3b00".to_string(),
            border: "#ffffff".to_string(),
            radius: 0.0,
            shadow: 4.0,
            font_size_base: 18.0,
        },
    ]
}
