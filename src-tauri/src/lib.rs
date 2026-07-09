mod fileops;
mod ipc;
mod scanner;
mod treemap;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(ipc::ScanState::default())
        .manage(treemap::TreemapState::default())
        .invoke_handler(tauri::generate_handler![
            ipc::app_info,
            ipc::list_volumes,
            ipc::start_scan,
            ipc::cancel_scan,
            ipc::scan_summary,
            ipc::get_children,
            ipc::node_path,
            treemap::treemap_render,
            treemap::treemap_meta,
            treemap::treemap_hit,
            fileops::open_item,
            fileops::reveal_item,
            fileops::trash_item,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Diskovery");
}
