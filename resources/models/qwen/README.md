Chatty-EDU does not include model weights. This folder documents supported third-party models and their licenses.

Purpose
- Clarify that no GGUF files are shipped in this repository; bring your own model.
- Provide attribution/guidance for Qwen models only; no binaries are included here.
- Point to vendor sources so you can fetch models yourself and keep deployments offline.

How to obtain
- Chatty-EDU does not download or manage models automatically.
- Download an appropriate Qwen GGUF from the official model host (e.g., Hugging Face Qwen team pages) under their published license/usage terms.
- Verify the license and any usage constraints before deploying in your environment.

Expected placement/format
- Chatty-EDU looks for user-provided GGUF files under your data path (default `data/models/` or `--base-path <dir>/models/`).
- Recommended filename pattern: `qwen2.5-<size>-instruct-<quant>.gguf` (example: `qwen2.5-1.5b-instruct-q4_k_m.gguf`).
- After copying the file into your models directory, select it in the app via File -> Models.

Offline stance
- Model selection and inference run locally; no network calls are made by Chatty-EDU during model use.
- Keep your deployments offline and manage model updates/approvals according to your org’s policy.***
