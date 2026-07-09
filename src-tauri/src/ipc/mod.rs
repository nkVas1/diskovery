use crate::scanner::{self, volumes, ExtStat, Progress, ScanTree};
use parking_lot::RwLock;
use serde::Serialize;
use std::cmp::Reverse;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

#[derive(Default)]
pub struct ScanState {
    pub tree: RwLock<Option<Arc<ScanTree>>>,
    pub running: AtomicBool,
    pub cancel: Arc<AtomicBool>,
}

#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ScanEvent {
    Progress {
        files: u64,
        dirs: u64,
        bytes: u64,
        errors: u64,
        current_path: String,
        elapsed_ms: u64,
    },
    Done {
        files: u64,
        dirs: u64,
        bytes: u64,
        errors: u64,
        elapsed_ms: u64,
    },
    Error {
        message: String,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDto {
    pub id: u32,
    pub name: String,
    pub size: u64,
    pub mtime: i64,
    pub is_dir: bool,
    pub child_count: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDto {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub size: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub root_path: String,
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub errors: u64,
    pub elapsed_ms: u64,
    pub top_dirs: Vec<NodeDto>,
    pub top_files: Vec<FileDto>,
    pub top_exts: Vec<ExtStat>,
}

#[derive(Serialize)]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
}

fn node_dto(tree: &ScanTree, id: u32) -> NodeDto {
    let n = &tree.nodes[id as usize];
    NodeDto {
        id,
        name: n.name.to_string(),
        size: n.size,
        mtime: n.mtime,
        is_dir: n.is_dir(),
        child_count: n.child_count,
    }
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: "Diskovery",
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[tauri::command]
pub fn list_volumes() -> Vec<volumes::VolumeInfo> {
    volumes::list()
}

#[tauri::command]
pub fn start_scan(app: AppHandle, path: String, on_event: Channel<ScanEvent>) -> Result<(), String> {
    let state = app.state::<ScanState>();
    let target = PathBuf::from(&path);
    if !target.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }
    if state.running.swap(true, Ordering::SeqCst) {
        return Err("A scan is already running".into());
    }
    state.cancel.store(false, Ordering::SeqCst);

    let cancel = state.cancel.clone();
    std::thread::spawn(move || {
        let started = Instant::now();
        let prog = Arc::new(Progress::default());
        let done = Arc::new(AtomicBool::new(false));

        // Progress ticker: streams counters to the UI every ~90 ms.
        let ticker = {
            let prog = prog.clone();
            let done = done.clone();
            let ch = on_event.clone();
            std::thread::spawn(move || {
                while !done.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(90));
                    let _ = ch.send(ScanEvent::Progress {
                        files: prog.files.load(Ordering::Relaxed),
                        dirs: prog.dirs.load(Ordering::Relaxed),
                        bytes: prog.bytes.load(Ordering::Relaxed),
                        errors: prog.errors.load(Ordering::Relaxed),
                        current_path: prog.current.lock().clone(),
                        elapsed_ms: started.elapsed().as_millis() as u64,
                    });
                }
            })
        };

        let tree = scanner::scan(target, &prog, &cancel);
        done.store(true, Ordering::Relaxed);
        let _ = ticker.join();

        let state = app.state::<ScanState>();
        if cancel.load(Ordering::SeqCst) {
            state.running.store(false, Ordering::SeqCst);
            let _ = on_event.send(ScanEvent::Error {
                message: "cancelled".into(),
            });
            return;
        }

        let event = ScanEvent::Done {
            files: tree.files,
            dirs: tree.dirs,
            bytes: tree.bytes,
            errors: tree.errors,
            elapsed_ms: tree.elapsed_ms,
        };
        *state.tree.write() = Some(Arc::new(tree));
        state.running.store(false, Ordering::SeqCst);
        let _ = on_event.send(event);
    });

    Ok(())
}

#[tauri::command]
pub fn cancel_scan(state: State<'_, ScanState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

fn current_tree(state: &State<'_, ScanState>) -> Result<Arc<ScanTree>, String> {
    state.tree.read().clone().ok_or_else(|| "No scan available".into())
}

#[tauri::command]
pub fn scan_summary(state: State<'_, ScanState>) -> Result<ScanSummary, String> {
    let tree = current_tree(&state)?;
    let root = &tree.nodes[0];

    let mut top_dirs: Vec<u32> = (root.child_start..root.child_start + root.child_count)
        .filter(|&i| tree.nodes[i as usize].is_dir())
        .collect();
    top_dirs.sort_unstable_by_key(|&i| Reverse(tree.nodes[i as usize].size));
    top_dirs.truncate(12);

    let top_files: Vec<FileDto> = tree
        .top_files
        .iter()
        .take(15)
        .map(|&id| {
            let n = &tree.nodes[id as usize];
            FileDto {
                id,
                name: n.name.to_string(),
                path: tree.path_of(n.parent).display().to_string(),
                size: n.size,
            }
        })
        .collect();

    Ok(ScanSummary {
        root_path: tree.root_path.display().to_string(),
        files: tree.files,
        dirs: tree.dirs,
        bytes: tree.bytes,
        errors: tree.errors,
        elapsed_ms: tree.elapsed_ms,
        top_dirs: top_dirs.into_iter().map(|id| node_dto(&tree, id)).collect(),
        top_files,
        top_exts: tree.ext_stats.iter().take(20).cloned().collect(),
    })
}

#[tauri::command]
pub fn get_children(state: State<'_, ScanState>, id: u32) -> Result<Vec<NodeDto>, String> {
    let tree = current_tree(&state)?;
    let n = tree
        .nodes
        .get(id as usize)
        .ok_or_else(|| "Unknown node".to_string())?;
    let mut ids: Vec<u32> = (n.child_start..n.child_start + n.child_count).collect();
    ids.sort_unstable_by_key(|&i| Reverse(tree.nodes[i as usize].size));
    Ok(ids.into_iter().map(|i| node_dto(&tree, i)).collect())
}

#[tauri::command]
pub fn node_path(state: State<'_, ScanState>, id: u32) -> Result<String, String> {
    let tree = current_tree(&state)?;
    if id as usize >= tree.nodes.len() {
        return Err("Unknown node".into());
    }
    Ok(tree.path_of(id).display().to_string())
}
