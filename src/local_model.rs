use crate::settings::ModelConfig;
use llama_cpp::{standard_sampler::StandardSampler, LlamaModel, LlamaParams, SessionParams};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::runtime::{Builder, Runtime};

#[derive(Clone)]
struct LoadedModel {
    path: PathBuf,
    model: Arc<LlamaModel>,
}

static MODEL: Lazy<RwLock<Option<LoadedModel>>> = Lazy::new(|| RwLock::new(None));
static TOKIO_RUNTIME: Lazy<parking_lot::Mutex<Runtime>> = Lazy::new(|| {
    parking_lot::Mutex::new(
        Builder::new_current_thread()
            .build()
            .expect("Failed to build Tokio runtime"),
    )
});

pub fn clear_cached_model() {
    MODEL.write().take();
}

fn load_model(path: &Path) -> Result<Arc<LlamaModel>, String> {
    if !path.exists() {
        return Err(format!("Model file not found: {}", path.display()));
    }

    let mut params = LlamaParams::default();
    params.n_gpu_layers = 0; // CPU only for school devices
    params.use_mmap = true;
    params.use_mlock = false;

    let model = LlamaModel::load_from_file(path, params)
        .map_err(|e| format!("Failed to load model {}: {e}", path.display()))?;

    Ok(Arc::new(model))
}

fn get_or_load_model(cfg: &ModelConfig) -> Result<Arc<LlamaModel>, String> {
    let wanted_path = PathBuf::from(&cfg.path);

    {
        let guard = MODEL.read();
        if let Some(current) = guard.as_ref() {
            if current.path == wanted_path {
                return Ok(current.model.clone());
            }
        }
    }

    let model = load_model(&wanted_path)?;
    let mut guard = MODEL.write();
    *guard = Some(LoadedModel {
        path: wanted_path,
        model: model.clone(),
    });
    Ok(model)
}

pub fn chat_completion(cfg: &ModelConfig, user_input: &str) -> Result<String, String> {
    let model = get_or_load_model(cfg)?;

    let mut session_params = SessionParams::default();
    // Keep context modest for low-end machines while allowing a reasonable history window.
    session_params.n_ctx = session_params.n_ctx.max(2048);
    session_params.n_batch = session_params.n_batch.max(256);
    session_params.n_ubatch = session_params.n_ubatch.max(128);
    session_params.n_threads = session_params.n_threads.max(1);
    session_params.n_threads_batch = session_params.n_threads_batch.max(1);

    let mut session = model
        .create_session(session_params)
        .map_err(|e| format!("Failed to create model session: {e}"))?;

    // Minimal system prompt to keep answers friendly and concise for students.
    let system_prompt =
        "You are Chatty-EDU, an offline school AI helper. Answer plainly, safely, and briefly.";
    let prompt = format!("{system_prompt}\n\nUser: {user_input}\nAssistant:");

    session
        .advance_context(prompt.as_bytes())
        .map_err(|e| format!("Could not feed prompt into model: {e}"))?;

    let max_predictions = cfg.max_tokens.max(16) as usize;
    let handle = session
        .start_completing_with(StandardSampler::default(), max_predictions)
        .map_err(|e| format!("Model could not start completion: {e}"))?;

    let output = TOKIO_RUNTIME.lock().block_on(handle.into_string_async());

    let cleaned = output.trim().to_string();
    if cleaned.is_empty() {
        Err("Model returned an empty response".to_string())
    } else {
        Ok(cleaned)
    }
}
