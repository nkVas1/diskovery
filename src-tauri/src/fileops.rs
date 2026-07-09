use crate::ipc::ScanState;
use crate::scanner::FLAG_DELETED;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

fn path_for(state: &State<'_, ScanState>, id: u32) -> Result<String, String> {
    let guard = state.tree.read();
    let tree = guard.as_ref().ok_or("No scan available")?;
    if id as usize >= tree.nodes.len() {
        return Err("Unknown node".into());
    }
    Ok(tree.path_of(id).display().to_string())
}

#[tauri::command]
pub fn open_item(app: AppHandle, state: State<'_, ScanState>, id: u32) -> Result<(), String> {
    let path = path_for(&state, id)?;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reveal_item(app: AppHandle, state: State<'_, ScanState>, id: u32) -> Result<(), String> {
    let path = path_for(&state, id)?;
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|e| e.to_string())
}

/// Move an item to the Recycle Bin and subtract its size from all ancestors.
/// The node is flagged deleted so layouts and listings skip it.
#[tauri::command]
pub fn trash_item(state: State<'_, ScanState>, id: u32) -> Result<u64, String> {
    let path = path_for(&state, id)?;
    trash::delete(&path).map_err(|e| e.to_string())?;

    let mut guard = state.tree.write();
    let tree = guard.as_mut().ok_or("No scan available")?;
    let removed = tree.nodes[id as usize].size;
    tree.nodes[id as usize].size = 0;
    tree.nodes[id as usize].flags |= FLAG_DELETED;
    let mut cur = id;
    while cur != 0 {
        cur = tree.nodes[cur as usize].parent;
        let n = &mut tree.nodes[cur as usize];
        n.size = n.size.saturating_sub(removed);
    }
    tree.bytes = tree.bytes.saturating_sub(removed);
    Ok(removed)
}
