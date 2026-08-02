//! Notebook save and open: native dialogs via rfd, plain std::fs IO.
//! Filled in by the persistence workstream.

#[tauri::command]
pub async fn notebook_save(_content: String, _path: Option<String>) -> Result<Option<String>, String> {
    Err("persistence not yet implemented".to_string())
}

#[tauri::command]
pub async fn notebook_open() -> Result<Option<(String, String)>, String> {
    Err("persistence not yet implemented".to_string())
}
