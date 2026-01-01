use std::path::{Path, PathBuf};

use crate::gui::models::ModuleManifest;

#[derive(Debug, Clone)]
pub struct LoadedModule {
    pub root: PathBuf,
    pub manifest: ModuleManifest,
}

pub fn scan_modules(base: &Path) -> Vec<LoadedModule> {
    let modules_dir = base.join("modules");
    let mut out = vec![];

    let subdirs = match std::fs::read_dir(&modules_dir) {
        Ok(v) => v,
        Err(_) => return out,
    };

    for entry in subdirs.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let manifest_path = dir.join("module.json");
        let raw = match std::fs::read_to_string(&manifest_path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let manifest: ModuleManifest = match serde_json::from_str(&raw) {
            Ok(m) => m,
            Err(_) => continue,
        };
        out.push(LoadedModule { root: dir, manifest });
    }

    out.sort_by(|a, b| {
        let ao = a.manifest.order.unwrap_or(9999);
        let bo = b.manifest.order.unwrap_or(9999);
        ao.cmp(&bo).then_with(|| a.manifest.title.cmp(&b.manifest.title))
    });

    out
}
