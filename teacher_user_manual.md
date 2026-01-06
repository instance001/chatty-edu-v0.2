# Chatty-EDU Teacher Manual (v0.4)

Audience: teachers and school IT. Everything runs offline by default; no accounts or cloud calls.

## What you need
- Windows PC (first target).
- Rust toolchain (`https://rustup.rs`) and LLVM/Clang (`LIBCLANG_PATH` set to its `bin`) if building.
- Optional: USB for portable data (`--base-path <USB path>`).

## First run (GUI, recommended)
1) Build/run: `cargo run -- --mode gui` (add `--base-path ...` for a USB path).
2) Models (offline AI):
   - Bring your own GGUF model (none is included in this repo).
   - Drop any GGUF into `data/models/`, then File -> Models to select. Large models may fail today; better handling is planned.
   - Model guidance/licensing notes: see `resources/models/` (e.g., `resources/models/qwen/README.md`).
3) Teacher lock:
   - Default PIN `0000`. Teacher menu → unlock with PIN (or secret answer if set).
   - While unlocked: change PIN, set secret question/answer, adjust game and hint settings. Lock when done.
4) Import/build packs:
  - Home tab → “Import pack file” (copies to `data/homework/assigned/`). Sample pack: `resources/homework_pack_sample_bundle.json` (copy into your data folder or import directly). If you use the sample attachment, copy `resources/attachments/` alongside the pack.
   - Or use Pack builder to create/export a pack.
5) Review + tutor:
   - Home tab + Homework Dashboard: assignments, filters, submissions, metrics; Teacher menu shows submissions summary.
   - Homework & Revision module: “Ask for hints” + “LLM homework helper” tied to selected assignment; hints-only mode is teacher-configurable.
   - Submissions are written to `data/homework/completed/` and include a hash-chained event log (start/answer/hint/retry/finalize) plus a final_hash for tamper-evidence and telemetry.

## Homework basics
- Packs are JSON (`homework_pack_*.json`). Place/import into `homework/assigned/`.
- Students: select assignment, fill “Submit work,” attach files if allowed, then “Export submission file” → `submission_<assignment_id>_<student>.json` in `homework/completed/`.
- Collect student submissions and place them in your `homework/completed/`; click “Rescan packs + submissions.”

## Revision basics
- Students can reopen any pack to practice.
- Tutor (in Homework & Revision) and Chat can give hints/steps, not full answers; hints-only mode can be enforced.

## CLI admin (quick)
`cargo run -- --mode cli`
- Enter teacher console: type `teacher`, PIN (default 0000; `forgot` to use secret answer).
- Commands: `create_pack`, `create_pack_multi`, `export_pack_template`, `import_pack <path>`, `import_submissions`, `show_completed`, `mode class`, `mode free`, `games on/off`, `allow_games_in_class`, `forbid_games_in_class`, `set_pin`, `set_secret`, `back`.
- Outside console: `import_pack <path>`, `submit <assignment_id>`.

## Data layout (under `./data` or `--base-path`)
- `config/` settings/UI, `homework/assigned/` packs, `homework/completed/` submissions, `models/` GGUF files, `modules/` manifests, `themes/`, `runtime/`, `logs/`, `revision/`, `ide/`.

## Safety/offline
- Offline-first; no network calls in core flows.
- Content filter (Janet) on by default; external process modules disabled unless allowed.

## Troubleshooting
- Build errors: install LLVM/Clang, set `LIBCLANG_PATH`, rerun `cargo build`.
- Missing packs/submissions: “Rescan packs + submissions” and confirm correct `--base-path`.
- PIN issues: use Teacher menu or CLI `teacher` → `forgot` (secret answer), then set a new PIN.
