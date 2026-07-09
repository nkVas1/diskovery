pub mod volumes;

use parking_lot::Mutex;
use rayon::prelude::*;
use serde::Serialize;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const FLAG_DIR: u8 = 1;
pub const FLAG_REPARSE: u8 = 2;
pub const FLAG_DELETED: u8 = 4;

/// One filesystem entry in the arena. Children of a directory occupy a
/// contiguous range `[child_start, child_start + child_count)`, so listing
/// a folder is a slice, not a traversal.
pub struct Node {
    pub name: Box<str>,
    pub parent: u32,
    pub size: u64,
    pub mtime: i64,
    pub flags: u8,
    pub child_start: u32,
    pub child_count: u32,
}

impl Node {
    pub fn is_dir(&self) -> bool {
        self.flags & FLAG_DIR != 0
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExtStat {
    pub ext: String,
    pub bytes: u64,
    pub count: u64,
}

pub struct ScanTree {
    pub nodes: Vec<Node>,
    pub root_path: PathBuf,
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub errors: u64,
    pub elapsed_ms: u64,
    /// All extensions, sorted by bytes desc.
    pub ext_stats: Vec<ExtStat>,
    /// Largest file node ids, sorted by size desc (top 100).
    pub top_files: Vec<u32>,
}

impl ScanTree {
    pub fn path_of(&self, id: u32) -> PathBuf {
        let mut parts: Vec<&str> = Vec::new();
        let mut cur = id;
        while cur != 0 {
            let n = &self.nodes[cur as usize];
            parts.push(&n.name);
            cur = n.parent;
        }
        let mut p = self.root_path.clone();
        for part in parts.iter().rev() {
            p.push(part);
        }
        p
    }
}

#[derive(Default)]
pub struct Progress {
    pub files: AtomicU64,
    pub dirs: AtomicU64,
    pub bytes: AtomicU64,
    pub errors: AtomicU64,
    pub current: Mutex<String>,
}

struct RawFile {
    name: Box<str>,
    size: u64,
    mtime: i64,
    reparse: bool,
}

struct RawDir {
    name: Box<str>,
    mtime: i64,
    size: u64,
    files: Vec<RawFile>,
    dirs: Vec<RawDir>,
}

fn unix_secs(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Recursive parallel walk. Reparse points (symlinks, junctions) are recorded
/// as zero-size leaves and never followed — no cycles, no double counting.
fn walk(path: PathBuf, name: Box<str>, mtime: i64, prog: &Progress, cancel: &AtomicBool) -> RawDir {
    if cancel.load(Ordering::Relaxed) {
        return RawDir {
            name,
            mtime,
            size: 0,
            files: Vec::new(),
            dirs: Vec::new(),
        };
    }
    if let Some(mut cur) = prog.current.try_lock() {
        *cur = path.display().to_string();
    }

    let mut files = Vec::new();
    let mut subdirs: Vec<(PathBuf, Box<str>, i64)> = Vec::new();
    let mut bytes_here = 0u64;

    match fs::read_dir(&path) {
        Ok(rd) => {
            for entry in rd {
                let Ok(entry) = entry else {
                    prog.errors.fetch_add(1, Ordering::Relaxed);
                    continue;
                };
                let Ok(ft) = entry.file_type() else {
                    prog.errors.fetch_add(1, Ordering::Relaxed);
                    continue;
                };
                let ename: Box<str> = entry.file_name().to_string_lossy().into_owned().into_boxed_str();
                if ft.is_symlink() {
                    files.push(RawFile {
                        name: ename,
                        size: 0,
                        mtime: 0,
                        reparse: true,
                    });
                } else if ft.is_dir() {
                    let mt = entry
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(unix_secs)
                        .unwrap_or(0);
                    subdirs.push((entry.path(), ename, mt));
                } else {
                    let (size, mt) = match entry.metadata() {
                        Ok(m) => (m.len(), m.modified().ok().map(unix_secs).unwrap_or(0)),
                        Err(_) => {
                            prog.errors.fetch_add(1, Ordering::Relaxed);
                            (0, 0)
                        }
                    };
                    bytes_here += size;
                    files.push(RawFile {
                        name: ename,
                        size,
                        mtime: mt,
                        reparse: false,
                    });
                }
            }
        }
        Err(_) => {
            prog.errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    prog.files.fetch_add(files.len() as u64, Ordering::Relaxed);
    prog.bytes.fetch_add(bytes_here, Ordering::Relaxed);
    prog.dirs.fetch_add(subdirs.len() as u64, Ordering::Relaxed);

    let dirs: Vec<RawDir> = subdirs
        .into_par_iter()
        .map(|(p, n, mt)| walk(p, n, mt, prog, cancel))
        .collect();

    let size = bytes_here + dirs.iter().map(|d| d.size).sum::<u64>();
    RawDir {
        name,
        mtime,
        size,
        files,
        dirs,
    }
}

fn ext_of(name: &str) -> Option<String> {
    let (stem, ext) = name.rsplit_once('.')?;
    if stem.is_empty() || ext.is_empty() || ext.len() > 12 || ext.contains(' ') {
        return None;
    }
    Some(ext.to_ascii_lowercase())
}

/// Walk `root` and flatten the result into an arena with BFS layout
/// (children contiguous, parents always before children).
pub fn scan(root: PathBuf, prog: &Progress, cancel: &AtomicBool) -> ScanTree {
    let started = std::time::Instant::now();
    let root_name: Box<str> = root.display().to_string().into_boxed_str();
    let raw = walk(root.clone(), root_name, 0, prog, cancel);

    let capacity = (prog.files.load(Ordering::Relaxed) + prog.dirs.load(Ordering::Relaxed) + 1) as usize;
    let mut nodes: Vec<Node> = Vec::with_capacity(capacity);
    nodes.push(Node {
        name: raw.name,
        parent: 0,
        size: raw.size,
        mtime: raw.mtime,
        flags: FLAG_DIR,
        child_start: 0,
        child_count: 0,
    });

    let mut ext_map: HashMap<String, (u64, u64)> = HashMap::new();
    let mut top: BinaryHeap<Reverse<(u64, u32)>> = BinaryHeap::new();
    let (mut files_total, mut dirs_total, mut bytes_total) = (0u64, 0u64, 0u64);

    let mut queue: VecDeque<(Vec<RawFile>, Vec<RawDir>, u32)> = VecDeque::new();
    queue.push_back((raw.files, raw.dirs, 0));

    while let Some((rfiles, rdirs, idx)) = queue.pop_front() {
        let child_start = nodes.len() as u32;
        let child_count = (rfiles.len() + rdirs.len()) as u32;
        for f in rfiles {
            let id = nodes.len() as u32;
            files_total += 1;
            bytes_total += f.size;
            if !f.reparse {
                if let Some(ext) = ext_of(&f.name) {
                    let e = ext_map.entry(ext).or_default();
                    e.0 += f.size;
                    e.1 += 1;
                }
                top.push(Reverse((f.size, id)));
                if top.len() > 100 {
                    top.pop();
                }
            }
            nodes.push(Node {
                name: f.name,
                parent: idx,
                size: f.size,
                mtime: f.mtime,
                flags: if f.reparse { FLAG_REPARSE } else { 0 },
                child_start: 0,
                child_count: 0,
            });
        }
        for d in rdirs {
            let RawDir {
                name,
                mtime,
                size,
                files,
                dirs,
            } = d;
            let id = nodes.len() as u32;
            dirs_total += 1;
            nodes.push(Node {
                name,
                parent: idx,
                size,
                mtime,
                flags: FLAG_DIR,
                child_start: 0,
                child_count: 0,
            });
            queue.push_back((files, dirs, id));
        }
        nodes[idx as usize].child_start = child_start;
        nodes[idx as usize].child_count = child_count;
    }

    let mut ext_stats: Vec<ExtStat> = ext_map
        .into_iter()
        .map(|(ext, (bytes, count))| ExtStat { ext, bytes, count })
        .collect();
    ext_stats.sort_unstable_by_key(|e| Reverse(e.bytes));

    let mut top_files: Vec<(u64, u32)> = top.into_iter().map(|Reverse(t)| t).collect();
    top_files.sort_unstable_by_key(|&(size, _)| Reverse(size));

    ScanTree {
        nodes,
        root_path: root,
        files: files_total,
        dirs: dirs_total,
        bytes: bytes_total,
        errors: prog.errors.load(Ordering::Relaxed),
        elapsed_ms: started.elapsed().as_millis() as u64,
        ext_stats,
        top_files: top_files.into_iter().map(|(_, id)| id).collect(),
    }
}
