mod advisor;
mod dedup;
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
        .manage(dedup::DedupState::default())
        .manage(advisor::AdvisorState::default())
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
            dedup::start_dedup,
            dedup::cancel_dedup,
            dedup::dedup_results,
            advisor::advisor_analyze,
            advisor::advisor_clean,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Diskovery");
}
