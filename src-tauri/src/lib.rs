//! Mushroom — local-first personal knowledge and notes.
//!
//! Keep this file thin: it wires plugins, managed state, and the command
//! handler list, and nothing else. See `.kiro/steering/structure.md`.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("Mushroom failed to start");
}
