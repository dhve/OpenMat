//! LLM provider bridge: Anthropic API and local Ollama, called from the
//! webview via Tauri commands so no CORS or key exposure in the page.
//!
//! Two Tauri commands are exposed (see src-tauri/src/lib.rs):
//!   - `llm_complete`: send a system + user prompt to either provider and
//!     return the model's plain-text reply.
//!   - `llm_list_ollama_models`: list locally installed Ollama models, for
//!     the settings pane's model dropdown.
//!
//! Both providers are reached via raw HTTP through reqwest (no first-party
//! Rust SDK exists for either API), never through a JS `fetch()` in the
//! webview, so the Anthropic API key never touches the page and no CORS
//! configuration is needed for either provider.

const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const ANTHROPIC_MAX_TOKENS: u32 = 1024;

const OLLAMA_CHAT_URL: &str = "http://localhost:11434/api/chat";
const OLLAMA_TAGS_URL: &str = "http://localhost:11434/api/tags";

// --- Anthropic wire types -------------------------------------------------

#[derive(serde::Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(serde::Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<AnthropicMessage<'a>>,
}

#[derive(serde::Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(serde::Deserialize)]
struct AnthropicResponse {
    #[serde(default)]
    content: Vec<AnthropicContentBlock>,
    stop_reason: Option<String>,
}

#[derive(serde::Deserialize)]
struct AnthropicErrorBody {
    error: AnthropicErrorDetail,
}

#[derive(serde::Deserialize)]
struct AnthropicErrorDetail {
    message: String,
}

// --- Ollama wire types -----------------------------------------------------

#[derive(serde::Serialize)]
struct OllamaMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(serde::Serialize)]
struct OllamaChatRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaMessage<'a>>,
    stream: bool,
}

#[derive(serde::Deserialize)]
struct OllamaChatResponseMessage {
    content: String,
}

#[derive(serde::Deserialize)]
struct OllamaChatResponse {
    message: OllamaChatResponseMessage,
}

#[derive(serde::Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaModelEntry>,
}

#[derive(serde::Deserialize)]
struct OllamaModelEntry {
    name: String,
}

// --- Shared helpers ----------------------------------------------------

/// Turn a connection-level reqwest error into a plain message; a connection
/// refused for the Ollama host almost always means the daemon is not
/// running, so say that directly instead of surfacing a raw OS error.
fn describe_ollama_connect_error(err: &reqwest::Error) -> String {
    if err.is_connect() {
        "Could not connect to Ollama at localhost:11434. Ollama does not appear to be running \
         - start it and try again."
            .to_string()
    } else {
        format!("Failed to reach Ollama: {err}")
    }
}

// --- Anthropic ---------------------------------------------------------

async fn call_anthropic(model: &str, api_key: &str, system: &str, prompt: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let body = AnthropicRequest {
        model,
        max_tokens: ANTHROPIC_MAX_TOKENS,
        system,
        messages: vec![AnthropicMessage { role: "user", content: prompt }],
    };

    let response = client
        .post(ANTHROPIC_MESSAGES_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("Failed to reach the Anthropic API: {err}"))?;

    let status = response.status();
    let raw_body = response
        .text()
        .await
        .map_err(|err| format!("Failed to read the Anthropic API response: {err}"))?;

    if !status.is_success() {
        let message = serde_json::from_str::<AnthropicErrorBody>(&raw_body)
            .map(|parsed| parsed.error.message)
            .unwrap_or_else(|_| raw_body.clone());
        return Err(format!("Anthropic API error ({status}): {message}"));
    }

    let parsed: AnthropicResponse = serde_json::from_str(&raw_body)
        .map_err(|err| format!("Failed to parse the Anthropic API response: {err}"))?;

    if parsed.stop_reason.as_deref() == Some("refusal") {
        return Err("Anthropic declined to respond to this request (refusal).".to_string());
    }

    let text = parsed
        .content
        .iter()
        .filter(|block| block.block_type == "text")
        .filter_map(|block| block.text.as_deref())
        .collect::<Vec<_>>()
        .join("");

    Ok(text)
}

// --- Ollama ---------------------------------------------------------------

async fn call_ollama(model: &str, system: &str, prompt: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let mut messages = Vec::with_capacity(2);
    if !system.is_empty() {
        messages.push(OllamaMessage { role: "system", content: system });
    }
    messages.push(OllamaMessage { role: "user", content: prompt });

    let body = OllamaChatRequest { model, messages, stream: false };

    let response = client
        .post(OLLAMA_CHAT_URL)
        .json(&body)
        .send()
        .await
        .map_err(|err| describe_ollama_connect_error(&err))?;

    let status = response.status();
    let raw_body = response
        .text()
        .await
        .map_err(|err| format!("Failed to read the Ollama response: {err}"))?;

    if !status.is_success() {
        return Err(format!("Ollama error ({status}): {raw_body}"));
    }

    let parsed: OllamaChatResponse =
        serde_json::from_str(&raw_body).map_err(|err| format!("Failed to parse the Ollama response: {err}"))?;

    Ok(parsed.message.content)
}

async fn fetch_ollama_models() -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();

    let response = client
        .get(OLLAMA_TAGS_URL)
        .send()
        .await
        .map_err(|err| describe_ollama_connect_error(&err))?;

    let status = response.status();
    let raw_body = response
        .text()
        .await
        .map_err(|err| format!("Failed to read the Ollama response: {err}"))?;

    if !status.is_success() {
        return Err(format!("Ollama error ({status}): {raw_body}"));
    }

    let parsed: OllamaTagsResponse =
        serde_json::from_str(&raw_body).map_err(|err| format!("Failed to parse the Ollama response: {err}"))?;

    Ok(parsed.models.into_iter().map(|entry| entry.name).collect())
}

// --- Tauri commands ---------------------------------------------------------

#[tauri::command]
pub async fn llm_complete(
    provider: String,
    model: String,
    api_key: Option<String>,
    system: String,
    prompt: String,
) -> Result<String, String> {
    match provider.as_str() {
        "anthropic" => {
            let key = match api_key {
                Some(k) if !k.trim().is_empty() => k,
                _ => return Err("An Anthropic API key is required.".to_string()),
            };
            call_anthropic(&model, &key, &system, &prompt).await
        }
        "ollama" => call_ollama(&model, &system, &prompt).await,
        other => Err(format!("Unknown provider \"{other}\". Expected \"anthropic\" or \"ollama\".")),
    }
}

#[tauri::command]
pub async fn llm_list_ollama_models() -> Result<Vec<String>, String> {
    fetch_ollama_models().await
}

// --- Tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Ollama is a real local dependency, not a mock: these tests hit it
    /// directly and skip (rather than fail) when it is not reachable, so
    /// the suite stays green on machines without Ollama installed or
    /// running.
    async fn ollama_reachable() -> bool {
        reqwest::Client::new().get(OLLAMA_TAGS_URL).send().await.is_ok()
    }

    // This workstream is scoped to app/src-tauri/src/llm.rs only and may not
    // touch Cargo.toml, which enables tokio with only the "rt" feature. The
    // #[tokio::test] attribute macro needs the "macros" feature, so these
    // tests build a runtime by hand instead (the "rt-multi-thread"/"net"/
    // "time" features are already pulled in transitively by tauri/reqwest).
    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Runtime::new().expect("failed to build a tokio runtime for the test").block_on(future)
    }

    #[test]
    fn ollama_list_models_live() {
        block_on(async {
            if !ollama_reachable().await {
                eprintln!("skipping ollama_list_models_live: Ollama not reachable at localhost:11434");
                return;
            }

            let models = fetch_ollama_models().await.expect("listing Ollama models should succeed");
            assert!(!models.is_empty(), "expected at least one installed Ollama model");
        });
    }

    #[test]
    fn ollama_complete_live() {
        block_on(async {
            if !ollama_reachable().await {
                eprintln!("skipping ollama_complete_live: Ollama not reachable at localhost:11434");
                return;
            }

            let models = fetch_ollama_models().await.expect("listing Ollama models should succeed");
            let model = models
                .iter()
                .find(|name| name.starts_with("qwen2.5:0.5b") || name.starts_with("llama3.2:1b"))
                .or_else(|| models.first())
                .cloned()
                .expect("expected at least one installed Ollama model to test against");

            let result = llm_complete(
                "ollama".to_string(),
                model,
                None,
                "You are a terse assistant. Reply with exactly one word and nothing else.".to_string(),
                "Say hello.".to_string(),
            )
            .await
            .expect("ollama completion should succeed");

            assert!(!result.trim().is_empty(), "expected a non-empty completion");
        });
    }

    #[test]
    fn llm_complete_rejects_unknown_provider() {
        block_on(async {
            let result = llm_complete(
                "not-a-real-provider".to_string(),
                "some-model".to_string(),
                None,
                "system".to_string(),
                "prompt".to_string(),
            )
            .await;

            assert!(result.is_err());
            assert!(result.unwrap_err().contains("Unknown provider"));
        });
    }

    #[test]
    fn llm_complete_rejects_anthropic_without_key() {
        block_on(async {
            let result = llm_complete(
                "anthropic".to_string(),
                "claude-opus-5".to_string(),
                None,
                "system".to_string(),
                "prompt".to_string(),
            )
            .await;

            assert!(result.is_err());
            assert!(result.unwrap_err().contains("API key"));
        });
    }
}
