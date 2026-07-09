mod ipc;
mod scanner;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(ipc::ScanState::default())
        .invoke_handler(tauri::generate_handler![
            ipc::app_info,
            ipc::list_volumes,
            ipc::start_scan,
            ipc::cancel_scan,
            ipc::scan_summary,
            ipc::get_children,
            ipc::node_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Diskovery");
}
