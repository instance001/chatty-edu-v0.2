# Resources

Sample and template artifacts that are safe to share publicly. All IDs and names are placeholders; no student or private data is included.

- `homework_pack_example.json`: minimal homework pack example for quick schema reference (self-contained).
- `homework_pack_sample_bundle.json`: multi-assignment bundle; Reading Trends is self-contained, Soil Moisture uses the attached field notes.
- `submission_example.json`: example submission payload showing pre-mark structure.
- `attachments/soil_moisture_gradient.txt`: dummy field-notes attachment referenced by the Soil Moisture assignment.
- Model guidance: see `resources/models/qwen/README.md` for supported Qwen variants and licensing notes (no weights included; bring your own GGUF).

Usage tips:
- Copy a pack from this folder into your runtime data directory (e.g., `homework/assigned/`) before running the app, or import it via the GUI/CLI. If a pack references attachments, copy the `attachments/` folder alongside it.
- Keep real submissions and packs out of version control; use these samples or your own sanitized templates instead.
