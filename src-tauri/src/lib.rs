mod ipc;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![ipc::app_info])
        .run(tauri::generate_context!())
        .expect("error while running Diskovery");
}
