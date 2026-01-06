# Chatty-EDU v0.4

Offline, local-first learning assistant for schools. No cloud, no accounts, no tracking. Ships as a single Rust binary with an egui desktop shell (Windows first) plus a CLI mode. Licensed under AGPLv3.

Designed for schools and boards:
- Runs entirely on school hardware; bring your own offline model (GGUF).
- Teacher PIN is meant to be changed on first login; keep the teacher menu locked in student-facing setups.
- Districts can drop in their preferred models; start with the bundled small model, then replace as needed.
- Works without internet; external processes are disabled unless explicitly allowed.

Design intent and boundaries: see `DESIGN_INTENT.md`. Public-safe sample packs and submission templates live in `resources/`.

## What's new in v0.4
- Local CPU models: drop a GGUF file into `data/models/`, pick it via File ? Models, and chat without Ollama.
- Teacher lock: PIN-gated teacher dashboard (default PIN 0000), changeable PIN, and secret question/answer for recovery; student view is default.
- AI pre-mark now uses the selected local model.
- Homework helper: module tab includes a hints-only tutor tied to the selected assignment (configurable by teacher).
- GUI parity: teacher menu mirrors CLI (import/export packs, rescan, class/free modes, games on/off, submissions summary).
- Docs and manuals refreshed for zero-knowledge setup.

## Project layout (auto-created under `./data` or `--base-path`)
- `config/` – settings, UI state
- `homework/assigned/` – homework packs (`homework_pack_*.json`)
- `homework/completed/` – submissions (`submission_*.json`)
- `modules/` – module manifests (built-in Homework Dashboard is auto-generated)
- `themes/` – active theme + presets
- `models/` – drop offline GGUF model files; select via File → Models
- `runtime/`, `logs/`, `revision/`, `ide/` – reserved for expansion

## Prereqs
- Rust toolchain (`https://rustup.rs`).
- LLVM/Clang for `llama_cpp` (set `LIBCLANG_PATH` to your LLVM `bin` on Windows) so the local-model crate can build.

## Models (bundled + swap-in)
- Model binaries are not included in the repo; drop an approved GGUF into `data/models/` (or your chosen `--base-path`) and select it via File ? Models.
- Model-agnostic: drop in your preferred GGUF models and select them via File → Models; districts are expected to use their approved models.
- Large models may exceed current runtime limits (e.g., GPT-OSS 20B failed to load); better large-model handling is planned.
- Model guidance/attribution: see `resources/models/` (e.g., `resources/models/qwen/README.md`) for supported third-party variants and licensing notes; no weights are shipped.

## Build and run
```bash
cargo build
cargo build --release   # release binary at target/release/chatty-edu

# GUI (default)
cargo run -- --mode gui

# CLI
cargo run -- --mode cli

# Custom data location (e.g., USB)
cargo run -- --mode gui --base-path D:\ChattyData
```

## GUI overview
- Menus: File / View / Modules / Tools / Teacher / Settings / Help.
- Tabs: Home (packs, submissions, metrics), Chat, Settings, Homework Dashboard (module), Homework & Revision module with built-in tutor.
- Models: File ? Models to pick a GGUF from `data/models/` (or refresh after you drop one in).
- Teacher lock: Teacher menu ? unlock with PIN (default PIN 0000; intended to be changed on first teacher unlock) or secret answer (default answer Math; intended to be changed on first teacher unlock); change PIN and secret while unlocked. Teacher Dashboard is hidden until unlocked.
- Homework packs: import a pack JSON from Home or Teacher menu; filters by assignment/subject; Rescan to reload. Sample pack lives in `resources/homework_pack_sample_bundle.json` (copy into your data folder or import directly, along with `resources/attachments/` if you want the demo attachment).
- Submissions: type answers, add attachments, export submission JSON with a hash-chained event log (start/answer/hint/retry/finalize) and final_hash for tamper-evidence.
- Metrics: class/subject averages, per-student bars; multi-student selection; filters apply across Home and Dashboard; submissions summary in Teacher menu.
- Themes: switch via View; presets include classic_light, chalkboard_dark, high_contrast.
- Homework tutor: "Ask for hints" and "LLM homework helper" live in the Homework & Revision module; hints-only mode is configurable (teacher-only).

## CLI quick commands
- `import_pack <path>` – copy a pack into `homework/assigned/`, apply policy.
- `create_pack` / `create_pack_multi` – interactive pack builders.
- `submit <assignment_id>` – prompt for answers/attachments; writes submission JSON to `homework/completed/`.
- `teacher` – enter teacher console (default PIN 0000; intended to be changed on first teacher unlock); type `forgot` to answer the secret question (default answer Math; intended to be changed on first teacher unlock). Inside teacher console:
  - `create_pack`, `create_pack_multi`, `export_pack_template`
  - `import_pack <path>`, `import_submissions`, `show_completed`
  - Mode controls: `mode class`, `mode free`
  - Game controls: `games on/off`, `allow_games_in_class`, `forbid_games_in_class`
  - PIN: `set_pin` (enter twice to confirm)
  - Secret: `set_secret` (update secret question/answer)

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
Entry types: `builtin_panel`, `markdown`, `static_html` (external_process exists but is gated/disabled by default).

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
  "student_name": "Sample Student",
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
- Content filter (Janet) is enabled by default and operates entirely offline.
- Homework packs, submissions, and AI pre-mark outputs are stored locally as readable JSON files.


