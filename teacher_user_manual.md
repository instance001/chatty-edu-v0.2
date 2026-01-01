# Chatty-EDU Teacher Manual (v0.3)

Audience: Teachers and school IT with no prior experience. Everything runs offline by default; no accounts or cloud.

## 1) What you need
- A Windows PC (builds target Windows first).
- Rust toolchain (install via https://rustup.rs).
- Optional: a USB stick if you want a fully portable data folder.

## 2) Install and run
1. Download or clone the repo.
2. Open a terminal in the project folder.
3. Build and run GUI (default):  
   ```bash
   cargo run -- --mode gui
   ```
   Run CLI instead:  
   ```bash
   cargo run -- --mode cli
   ```
4. Data location: by default, the app uses `./data` next to the executable. To use a custom location (USB or network drive), add `--base-path <path>`:
   ```bash
   cargo run -- --mode gui --base-path D:\ChattyData
   ```

## 3) Data layout (auto-created)
- `config/` – settings, UI state
- `homework/assigned/` – homework packs to distribute (`homework_pack_*.json`)
- `homework/completed/` – student submissions (`submission_*.json`)
- `modules/` – module manifests (a built-in Homework Dashboard is auto-created)
- `themes/` – active theme + presets
- `runtime/`, `logs/`, `revision/`, `ide/` – reserved for expansion

## 4) Key concepts
- **Pack**: a JSON file with one or more assignments (title, subject, due date, games allowed, etc.).
- **Submission**: a JSON file a student exports after completing homework; can include attachments and auto pre-mark feedback.
- **Metrics**: in the GUI, view class/subject averages and per-student performance; filters for assignment, subject, and multi-student selection.
- **Modules**: menu of drop-in tools; the built-in Homework Dashboard ships by default.

## 5) Teacher CLI quick commands
Run CLI mode first: `cargo run -- --mode cli`
- `teacher` → enter teacher console (PIN is stubbed)
  - `create_pack` → guided single-assignment pack builder
  - `create_pack_multi` → guided multi-assignment builder
  - `export_pack_template` → writes a sample pack JSON to `homework/assigned/`
  - `import_pack <path>` → copy a pack into `homework/assigned/` and apply policy (games off if set)
  - `import_submissions` → summarize all `submission_*.json` in `homework/completed/`
  - `show_completed` / `homework table` → list completed homework (legacy CLI table)
  - Game controls: `games on/off`, `allow_games_in_class`, `forbid_games_in_class`
  - Mode controls: `mode class`, `mode free`
- Outside teacher console:
  - `submit <assignment_id>` → quick way to generate a submission (for testing)

## 6) Teacher GUI workflow (recommended)
Start GUI: `cargo run -- --mode gui`
- **Import a pack**: Home tab → “Import pack file…” (JSON). The app copies it to `homework/assigned/`, applies game policy, and rescans.
- **Build a pack**: Home tab → “Pack builder (teacher)” → fill assignment fields, add to pack, export.
- **Assign/filters**: Use assignment and subject dropdowns to filter metrics and lists.
- **Metrics**: View class and per-student averages; multi-select students for focused metrics.
- **Dashboard**: The Homework Dashboard module shows pack details, averages, and submission rows with titles, IDs, scores, and feedback.
- **Submissions**: Scroll lists on Home or Dashboard to see submission rows; scores prefer AI pre-mark; attachments are listed in the submission JSON (not displayed inline).

## 7) Moving files between teacher portal and students
- Publish `homework_pack_*.json` to your school portal/shared drive.
- Students download it into `homework/assigned/` (or use GUI import).
- Students work offline, then export `submission_*.json` (with optional attachments) into `homework/completed/`, and upload to the portal.
- Teachers download submissions and run `import_submissions` (CLI) or view them in the GUI/Dashboard.

## 8) Safety and offline stance
- Offline by default; no cloud calls in core flows.
- External process modules are disabled unless explicitly allowed.
- Content filter (“Janet”) is on by default to block swears/mature topics.

## 9) Troubleshooting
- Data not showing? Click “Rescan packs + submissions” on Home.
- Games not toggling? Pack policy may disable games; re-import settings or adjust in teacher console.
- Custom data path? Always pass `--base-path` to ensure you’re using the intended folder (e.g., USB).
- Build issues? Ensure Rust is installed via rustup and rerun `cargo build`.

## 10) License
AGPL-3.0-or-later (see `LICENSE`).
