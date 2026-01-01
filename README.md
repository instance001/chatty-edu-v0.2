# Chatty-EDU v0.3

Offline, local-first learning assistant for schools. No cloud, no accounts, no tracking. Ships as a single Rust binary with an egui desktop shell (Windows builds first), plus a CLI mode.

Key targets: primary/high-school teacher and student workflows with homework packs, teacher dashboard, and a modular add-on system. Licensed under AGPLv3.

📄 Design Intent: See DESIGN_INTENT.md for the system’s boundaries and ethical framing.

## What’s in v0.3
- GUI shell (eframe/egui) with menu + tabs, themes, and built-in Homework Dashboard.
- Homework packs: import/export, assignment and subject filters, per-student/class metrics.
- Submissions: students can export work with optional attachments; simple AI pre-mark stub adds score/feedback for quick triage.
- Module system: drop-in `modules/<id>/module.json` manifest; built-in Homework Dashboard is auto-generated.
- Portable data layout under `./data` by default; configurable `--base-path`.
- Safety: offline by default; external process modules gated; content filter (“Janet”) preserved.

## Project structure (data-oriented)
- `src/` — Rust sources (GUI shell, CLI, homework pack/submission logic, settings, themes, modules).
- `data/` (or `--base-path <path>`):
  - `config/` — settings, ui state
  - `homework/assigned/` — homework packs (`homework_pack_*.json`)
  - `homework/completed/` — submissions (`submission_*.json`)
  - `modules/` — module manifests (built-in dashboard created automatically)
  - `themes/` — active theme + presets
  - `runtime/`, `logs/`, `revision/`, `ide/` — future expansion

## Running
Requires Rust (rustup recommended).

```bash
cargo build
cargo build --release   # release binary at target/release/chatty-edu
cargo run -- --mode gui        # GUI (default)
cargo run -- --mode cli        # CLI mode
cargo run -- --mode gui --base-path D:\ChattyData   # custom data path
```

## GUI overview
- Menus: File / View / Modules / Tools / Settings / Help.
- Tabs: Home (packs, submissions, metrics), Chat, Settings, Homework Dashboard (module).
- Packs: Import a pack file (JSON) from Home; filters by assignment/subject; Rescan to reload.
- Submission export: type answers, attach files, export submission JSON for upload to the school portal.
- Metrics: class/subject averages, per-student bars; multi-student selection; filters apply across Home and Dashboard.
- Themes: switch via View; presets include classic_light, chalkboard_dark, high_contrast.

## CLI quick commands
- `import_pack <path>` — copy a pack into `homework/assigned/`, apply policy.
- `create_pack` / `create_pack_multi` — interactive pack builders.
- `submit <assignment_id>` — prompt for answers and optional attachments; writes submission JSON to `homework/completed/`.
- Teacher console: `teacher` (stub PIN) with commands for mode, games, pack export, submissions import.

## Module manifests (summary)
`modules/<id>/module.json`:
```json
{
  "id": "homework_dashboard",
  "title": "Homework Dashboard",
  "roles": ["teacher", "student"],
  "entry": { "type": "builtin_panel", "target": "homework_dashboard" },
  "version": "1.0.0",
  "description": "Built-in view for packs and submissions"
}
```
Supported entry types today: `builtin_panel`, `markdown`, `static_html`; `external_process` exists but is gated/disabled by default.

## Homework pack schema (v1)
```json
{
  "version": "1.0",
  "school_id": "school-123",
  "class_id": "yr7-math-a",
  "created_at": "2026-01-01T00:00:00Z",
  "assignments": [
    {
      "id": "hw-001",
      "title": "Fractions",
      "subject": "Math",
      "year_level": "7",
      "due_at": "2026-01-05T09:00:00Z",
      "instructions_md": "Solve the attached problems...",
      "allow_games": false,
      "allow_ai_premark": true,
      "max_score": 100,
      "attachments": []
    }
  ]
}
```

## Submission schema (v1)
```json
{
  "version": "1.0",
  "school_id": "school-123",
  "class_id": "yr7-math-a",
  "assignment_id": "hw-001",
  "student_id": "s12345",
  "student_name": "Ada L",
  "submitted_at": "2026-01-02T15:30:00Z",
  "answers_text": "My work...",
  "answers": [],
  "ai_premark": { "score": 78, "feedback": "Check step 3." },
  "attachments": ["path/to/work.pdf"]
}
```

## Safety and offline stance
- Offline by default; no network calls in core flows.
- External process modules are disabled unless explicitly allowed.
- Content filter (Janet) is enabled by default.

## License
AGPL-3.0-or-later (see `LICENSE`).
