# Chatty-EDU Student Manual (v0.4)

Audience: students with zero setup experience. Everything runs offline; no accounts.

## Quick start (prebuilt)
1) Open `chatty-edu.exe` (or run it from a terminal) in the folder provided to you.
2) Models: bring your own GGUF (none is bundled here). If your teacher gives you one, drop it in `data/models/` and choose it via File -> Models. Model guidance lives in `resources/models/` (e.g., `resources/models/qwen/README.md`).
3) Import homework: Home tab → “Import pack file” to load `homework_pack_*.json` (or copy it into `data/homework/assigned/`).
4) Pick your assignment in Home. Read the instructions.
5) Under “Submit work,” type your answers. Add attachments if your teacher asked.
6) Click “Export submission file” to save `submission_<assignment_id>_<your_id>.json` into `data/homework/completed/`. Upload that JSON (and any attachments) via your normal hand-in method.

## Quick start (build yourself)
1) Install Rust (`https://rustup.rs`) and LLVM/Clang (set `LIBCLANG_PATH` to its `bin` folder).
2) In the project folder: `cargo run -- --mode gui` (or `--mode cli`).
3) Follow the same steps as “Quick start (prebuilt)” to import a pack and submit work.

## Homework & Revision
- Homework packs live in `data/homework/assigned/`. If you don’t see yours, click “Rescan packs + submissions” on Home.
- The Homework & Revision module has “Ask for hints” and an “LLM homework helper” tied to the selected assignment. These give hints, not full answers (teacher can configure hints-only).
- Chat tab is for general learning questions (still filtered for safety).

## File locations (auto-created under `data/`)
- `homework/assigned/` — homework packs (`homework_pack_*.json`)
- `homework/completed/` — your exported submissions (`submission_*.json`)
- `models/` — local GGUF models; pick via File → Models
- `config/`, `themes/`, `modules/` — app settings/themes (usually leave alone)

## Tips
- If you change computers, keep the whole `data/` folder with you (USB-friendly). Run with `--base-path <USB path>` to force data onto the USB.
- If the tutor says it can’t give the answer, ask for steps or key ideas instead.
- Teacher-only settings (filters, games, hints-only toggle) are locked behind the Teacher PIN; students can’t change them.

## Staying offline and safe
- Chatty-EDU works without internet; no cloud calls in normal use.
- Content filter (Janet) is on by default to block swears/mature topics.
