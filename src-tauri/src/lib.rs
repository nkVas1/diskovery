mod advisor;
mod ai;
mod dedup;
mod fileops;
mod ipc;
mod scanner;
mod settings;
mod treemap;

use parking_lot::RwLock;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(ipc::ScanState::default())
        .manage(treemap::TreemapState::default())
        .manage(dedup::DedupState::default())
        .manage(advisor::AdvisorState::default())
        .manage(ai::AiState::default())
        .setup(|app| {
            let loaded = settings::load(app.handle());
            app.manage(settings::SettingsState(RwLock::new(loaded)));
            Ok(())
        })
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
            settings::get_settings,
            settings::set_settings,
            ai::ai_digest_preview,
            ai::ai_analyze,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Diskovery");
}
