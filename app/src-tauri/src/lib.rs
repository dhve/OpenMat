// App-to-kernel contract (ARCHITECTURE.md): one command, evaluate(input) ->
// EvalResult. Field names match the TypeScript EvalResult type in
// src/engine/types.ts exactly (x_range, y_range stay snake_case).
//
// This is a placeholder until openmat-kernel is wired in. The UI runs on
// its own TypeScript mockEngine today (src/engine/mockEngine.ts) and does
// not call this command; it exists so the IPC shape is already in place for
// integration, and so the desktop shell has something real to compile and
// serialize.

use serde::Serialize;

#[derive(Serialize)]
struct Curve {
    points: Vec<(f64, f64)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
}

#[derive(Serialize)]
struct PlotData {
    curves: Vec<Curve>,
    x_range: (f64, f64),
    y_range: (f64, f64),
}

#[derive(Serialize)]
struct EvalResult {
    latex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    plot: Option<PlotData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[tauri::command]
fn evaluate(input: String) -> EvalResult {
    let _ = input;
    EvalResult {
        latex: String::new(),
        plot: None,
        error: Some("openmat-kernel is not wired up yet; the UI runs on its own mock engine.".to_string()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![evaluate])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
