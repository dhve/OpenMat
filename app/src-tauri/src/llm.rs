//! LLM provider bridge: Anthropic API and local Ollama, called from the
//! webview via Tauri commands so no CORS or key exposure in the page.
//! Filled in by the NL-input workstream.

#[tauri::command]
pub async fn llm_complete(
    _provider: String,
    _model: String,
    _api_key: Option<String>,
    _system: String,
    _prompt: String,
) -> Result<String, String> {
    Err("llm backend not yet implemented".to_string())
}

#[tauri::command]
pub async fn llm_list_ollama_models() -> Result<Vec<String>, String> {
    Err("llm backend not yet implemented".to_string())
}
