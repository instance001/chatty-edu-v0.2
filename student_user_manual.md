# Chatty-EDU Student Manual (v0.3)

Audience: Students with no prior setup experience. Everything runs offline; no accounts or cloud.

## 1) What you need
- A Windows PC (builds target Windows first).
- Rust toolchain (install via https://rustup.rs).
- If your teacher gives you a USB with Chatty-EDU, use it directly; otherwise, download/clone the project.

## 2) Install and run
1. Open a terminal in the project folder.
2. Build and run GUI (recommended):  
   ```bash
   cargo run -- --mode gui
   ```
   CLI mode (advanced):  
   ```bash
   cargo run -- --mode cli
   ```
3. Data location: by default, the app uses `./data` next to the executable. If using a USB, you can keep everything there or set `--base-path` to the USB path.

## 3) Where your files live
- `homework/assigned/` — place homework packs from your teacher here (files named `homework_pack_*.json`).
- `homework/completed/` — your exported submissions (files named `submission_*.json`).
- `themes/`, `config/`, `modules/` — app settings/themes; you normally don’t edit these.

## 4) Basic flow (GUI)
1. Get a homework pack (JSON) from your teacher/portal.
2. Start Chatty-EDU GUI: `cargo run -- --mode gui`.
3. Home tab → “Import pack file…” and pick the pack; it will copy into `homework/assigned/`.
4. Open the assignment list (filters by assignment/subject are available). Select the assignment you’re working on.
5. In “Submit work,” type your answers. Click “Add attachments…” if you need to include files (photos, PDFs, etc.).
6. Click “Export submission file” to save a `submission_<assignment_id>_<your_id>.json` into `homework/completed/`.
7. Upload that submission file (and any attachments if required) to your school portal or hand-in method.

## 5) CLI quick commands (optional)
Run CLI mode: `cargo run -- --mode cli`
- `import_pack <path>` — copy a pack into `homework/assigned/`.
- `submit <assignment_id>` — prompt for answers and optional attachment paths (comma-separated) and save a submission JSON to `homework/completed/`.
- `exit` — quit.

## 6) Metrics view
In the GUI, the Home tab and the Homework Dashboard module show:
- Assignment and subject filters.
- Class/subject averages (for teachers); you can ignore these if just working solo.
- Per-student bars (if multiple students’ submissions are present in the same folder).

## 7) Staying offline and safe
- Chatty-EDU works without internet; no accounts.
- External add-on programs are disabled by default for safety.
- The content filter (“Janet”) blocks swears/mature topics by default.

## 8) Tips
- Keep your pack and submissions on the same USB if you want portability; run with `--base-path <USB path>` to keep everything together.
- Always upload the generated `submission_*.json` to your portal; attachments may also be needed if the teacher asked.
- If you can’t see your assignment, click “Rescan packs + submissions” in the Home tab.

## 9) License
AGPL-3.0-or-later (see `LICENSE`). You’re free to use and share within the license terms.
