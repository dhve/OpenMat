// App-to-kernel contract (ARCHITECTURE.md, "Kernel API"): evaluate(input,
// bindings, request_id) -> KernelResult. This is the local adapter
// (ARCHITECTURE.md, "Kernel service and transport adapters"): it owns no
// evaluation semantics or result formatting itself, it just calls
// openmat-kernel in-process and hands back what it returns.
//
// Tauri camelCases Rust parameter names by default when matching the JS
// invoke() payload, so `request_id` here is reached from the frontend as
// `requestId`; no `rename_all` needed. See src/engine/tauriEngine.ts for the
// JS side of this call.

mod files;
mod llm;

use std::collections::HashMap;

use openmat_kernel::KernelResult;

#[tauri::command]
fn evaluate(input: String, bindings: HashMap<String, f64>, request_id: u64) -> KernelResult {
    openmat_kernel::evaluate_with_bindings(&input, &bindings, request_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            evaluate,
            llm::llm_complete,
            llm::llm_list_ollama_models,
            files::notebook_save,
            files::notebook_open
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
