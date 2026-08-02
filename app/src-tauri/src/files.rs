//! Notebook save and open: native dialogs via rfd, plain std::fs IO.
//! Filled in by the persistence workstream.
//!
//! The TS side (src/persistence/) owns the .omnb JSON shape; this module
//! only moves bytes between the renderer and disk plus the native file
//! dialogs. `content` is already a fully serialized JSON string by the time
//! it reaches `notebook_save`, and the string returned from `notebook_open`
//! is handed back to the renderer unparsed.

const OMNB_EXTENSION: &str = "omnb";
const DEFAULT_FILE_NAME: &str = "Untitled.omnb";

#[tauri::command]
pub async fn notebook_save(content: String, path: Option<String>) -> Result<Option<String>, String> {
    let chosen_path = match path {
        Some(p) => p,
        None => {
            let file_handle = rfd::AsyncFileDialog::new()
                .add_filter("OpenMat Notebook", &[OMNB_EXTENSION])
                .set_file_name(DEFAULT_FILE_NAME)
                .save_file()
                .await;

            match file_handle {
                Some(handle) => handle.path().to_string_lossy().into_owned(),
                None => return Ok(None),
            }
        }
    };

    std::fs::write(&chosen_path, content).map_err(|err| err.to_string())?;

    Ok(Some(chosen_path))
}

#[tauri::command]
pub async fn notebook_open() -> Result<Option<(String, String)>, String> {
    let file_handle = rfd::AsyncFileDialog::new()
        .add_filter("OpenMat Notebook", &[OMNB_EXTENSION])
        .pick_file()
        .await;

    let handle = match file_handle {
        Some(handle) => handle,
        None => return Ok(None),
    };

    let chosen_path = handle.path().to_string_lossy().into_owned();
    let content = std::fs::read_to_string(&chosen_path).map_err(|err| err.to_string())?;

    Ok(Some((chosen_path, content)))
}
