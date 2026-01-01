use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub base_path: Option<String>,        // default: None => current dir
    pub default_theme: Option<String>,    // e.g., "classic_light.json"
    pub default_tabs: Option<Vec<String>> // e.g., ["home","chat"]
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            base_path: None,
            default_theme: Some("classic_light.json".to_string()),
            default_tabs: Some(vec!["home".into(), "chat".into()]),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub allow_external_exe_modules: bool,
    pub allow_network: bool,
    pub allow_file_export: bool,
    pub allow_clipboard: bool,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            allow_external_exe_modules: false,
            allow_network: false,
            allow_file_export: true,
            allow_clipboard: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: String,
    pub font_scale: Option<f32>,
    pub rounding: Option<f32>,
    pub spacing: Option<f32>,
    pub colors: ThemeColors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    pub bg: String,
    pub panel: String,
    pub text: String,
    pub muted_text: String,
    pub accent: String,
    pub danger: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub id: String,
    pub title: String,
    pub version: String,
    pub audience: Vec<String>,
    pub entry: ModuleEntry,
    pub icon: Option<String>,
    pub order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleEntry {
    #[serde(rename = "type")]
    pub kind: ModuleEntryType,

    // markdown
    pub path: Option<String>,

    // native_panel
    pub panel: Option<String>,

    // external_exe
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleEntryType {
    Markdown,
    NativePanel,
    ExternalExe,
}
