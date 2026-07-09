//! Duplicate detection: size groups → 16 KB BLAKE3 prehash → full BLAKE3,
//! with a persistent redb hash cache and NTFS hardlink awareness.

use crate::ipc::ScanState;
use crate::scanner::{FLAG_DELETED, FLAG_DIR, FLAG_REPARSE};
use parking_lot::RwLock;
use rayon::prelude::*;
use redb::{Database, TableDefinition};
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

const CACHE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("full_hashes_v1");
const PREHASH_LEN: usize = 16 * 1024;
const MMAP_THRESHOLD: u64 = 4 * 1024 * 1024;

pub struct DupGroup {
    pub size: u64,
    pub hash: [u8; 32],
    pub files: Vec<u32>,
}

pub struct DedupResult {
    pub groups: Vec<DupGroup>,
    pub hashed_bytes: u64,
    pub cache_hits: u64,
    pub elapsed_ms: u64,
}

#[derive(Default)]
pub struct DedupState {
    pub result: RwLock<Option<DedupResult>>,
    pub running: AtomicBool,
    pub cancel: Arc<AtomicBool>,
}

#[derive(Serialize, Clone)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DedupEvent {
    Progress {
        stage: &'static str,
        done: u64,
        total: u64,
    },
    Done {
        groups: u64,
        wasted_bytes: u64,
        hashed_bytes: u64,
        cache_hits: u64,
        elapsed_ms: u64,
    },
    Error {
        message: String,
    },
}

/* ---------------- hashing primitives ---------------- */

fn prehash_file(path: &Path) -> std::io::Result<[u8; 32]> {
    let mut f = File::open(path)?;
    let mut buf = [0u8; PREHASH_LEN];
    let mut read = 0usize;
    loop {
        let n = f.read(&mut buf[read..])?;
        if n == 0 || read + n == PREHASH_LEN {
            read += n;
            break;
        }
        read += n;
    }
    Ok(*blake3::hash(&buf[..read]).as_bytes())
}

fn hash_file(path: &Path, size: u64) -> std::io::Result<[u8; 32]> {
    let file = File::open(path)?;
    if size >= MMAP_THRESHOLD {
        // SAFETY: the file may change under us; a torn read only yields a
        // wrong hash for a file that was being written — acceptable here.
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        Ok(*blake3::hash(&mmap).as_bytes())
    } else {
        let mut buf = Vec::with_capacity(size as usize);
        let mut f = file;
        f.read_to_end(&mut buf)?;
        Ok(*blake3::hash(&buf).as_bytes())
    }
}

fn pack_cache(size: u64, mtime: i64, hash: &[u8; 32]) -> [u8; 48] {
    let mut out = [0u8; 48];
    out[..8].copy_from_slice(&size.to_le_bytes());
    out[8..16].copy_from_slice(&mtime.to_le_bytes());
    out[16..].copy_from_slice(hash);
    out
}

fn unpack_cache(v: &[u8]) -> Option<(u64, i64, [u8; 32])> {
    if v.len() != 48 {
        return None;
    }
    let size = u64::from_le_bytes(v[..8].try_into().ok()?);
    let mtime = i64::from_le_bytes(v[8..16].try_into().ok()?);
    let hash: [u8; 32] = v[16..].try_into().ok()?;
    Some((size, mtime, hash))
}

fn open_cache(app: &AppHandle) -> Option<Database> {
    let dir = app.path().app_local_data_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    Database::create(dir.join("hashcache.redb")).ok()
}

/* ---------------- pipeline ---------------- */

struct Candidate {
    id: u32,
    size: u64,
    mtime: i64,
    path: PathBuf,
}

#[allow(clippy::too_many_lines)]
fn run_pipeline(
    app: &AppHandle,
    min_size: u64,
    cancel: &AtomicBool,
    stage: &AtomicU8,
    done: &AtomicU64,
    total: &AtomicU64,
) -> Result<DedupResult, String> {
    let started = Instant::now();
    let scan = app.state::<ScanState>();

    // Stage 0: collect candidates grouped by size.
    let mut by_size: HashMap<u64, Vec<Candidate>> = HashMap::new();
    {
        let guard = scan.tree.read();
        let tree = guard.as_ref().ok_or("No scan available")?;
        for (i, n) in tree.nodes.iter().enumerate() {
            if n.flags & (FLAG_DIR | FLAG_REPARSE | FLAG_DELETED) != 0 || n.size < min_size.max(1) {
                continue;
            }
            by_size.entry(n.size).or_default().push(Candidate {
                id: i as u32,
                size: n.size,
                mtime: n.mtime,
                path: tree.path_of(i as u32),
            });
        }
    }
    let candidates: Vec<Candidate> = by_size
        .into_values()
        .filter(|v| v.len() >= 2)
        .flatten()
        .collect();

    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }

    // Stage 1: prehash (first 16 KB).
    stage.store(1, Ordering::Relaxed);
    done.store(0, Ordering::Relaxed);
    total.store(candidates.len() as u64, Ordering::Relaxed);

    let prehashes: Vec<Option<[u8; 32]>> = candidates
        .par_iter()
        .map(|c| {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            let r = prehash_file(&c.path).ok();
            done.fetch_add(1, Ordering::Relaxed);
            r
        })
        .collect();

    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }

    let mut by_prehash: HashMap<(u64, [u8; 32]), Vec<usize>> = HashMap::new();
    for (i, ph) in prehashes.iter().enumerate() {
        if let Some(ph) = ph {
            by_prehash
                .entry((candidates[i].size, *ph))
                .or_default()
                .push(i);
        }
    }
    let survivors: Vec<usize> = by_prehash
        .into_values()
        .filter(|v| v.len() >= 2)
        .flatten()
        .collect();

    // Stage 2: full hash with cache.
    stage.store(2, Ordering::Relaxed);
    done.store(0, Ordering::Relaxed);
    total.store(
        survivors.iter().map(|&i| candidates[i].size).sum(),
        Ordering::Relaxed,
    );

    let cache = open_cache(app);
    let mut full_hashes: Vec<Option<[u8; 32]>> = vec![None; candidates.len()];
    let mut to_hash: Vec<usize> = Vec::new();
    let mut cache_hits = 0u64;

    if let Some(db) = &cache {
        if let Ok(txn) = db.begin_read() {
            if let Ok(table) = txn.open_table(CACHE_TABLE) {
                for &i in &survivors {
                    let c = &candidates[i];
                    let key = c.path.display().to_string();
                    let hit = table.get(key.as_str()).ok().flatten().and_then(|v| {
                        let (size, mtime, hash) = unpack_cache(v.value())?;
                        (size == c.size && mtime == c.mtime).then_some(hash)
                    });
                    match hit {
                        Some(h) => {
                            full_hashes[i] = Some(h);
                            cache_hits += 1;
                            done.fetch_add(c.size, Ordering::Relaxed);
                        }
                        None => to_hash.push(i),
                    }
                }
            } else {
                to_hash = survivors.clone();
            }
        } else {
            to_hash = survivors.clone();
        }
    } else {
        to_hash = survivors.clone();
    }

    let hashed_bytes: u64 = to_hash.iter().map(|&i| candidates[i].size).sum();
    let hashed: Vec<(usize, Option<[u8; 32]>)> = to_hash
        .par_iter()
        .map(|&i| {
            if cancel.load(Ordering::Relaxed) {
                return (i, None);
            }
            let c = &candidates[i];
            let r = hash_file(&c.path, c.size).ok();
            done.fetch_add(c.size, Ordering::Relaxed);
            (i, r)
        })
        .collect();

    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }

    // Persist newly computed hashes.
    if let Some(db) = &cache {
        if let Ok(txn) = db.begin_write() {
            if let Ok(mut table) = txn.open_table(CACHE_TABLE) {
                for (i, h) in &hashed {
                    if let Some(h) = h {
                        let c = &candidates[*i];
                        let key = c.path.display().to_string();
                        let _ =
                            table.insert(key.as_str(), pack_cache(c.size, c.mtime, h).as_slice());
                    }
                }
            }
            let _ = txn.commit();
        }
    }
    for (i, h) in hashed {
        full_hashes[i] = h;
    }

    // Final grouping by (size, full hash).
    let mut by_hash: HashMap<(u64, [u8; 32]), Vec<usize>> = HashMap::new();
    for &i in &survivors {
        if let Some(h) = full_hashes[i] {
            by_hash.entry((candidates[i].size, h)).or_default().push(i);
        }
    }

    let mut groups: Vec<DupGroup> = Vec::new();
    for ((size, hash), members) in by_hash {
        if members.len() < 2 {
            continue;
        }
        // Hardlink filter: files sharing an NTFS file id are one physical
        // file, not duplicates. Handle compares by (volume, file index).
        let mut seen: Vec<same_file::Handle> = Vec::with_capacity(members.len());
        let mut files: Vec<u32> = Vec::with_capacity(members.len());
        for &i in &members {
            let c = &candidates[i];
            match same_file::Handle::from_path(&c.path) {
                Ok(h) => {
                    if !seen.contains(&h) {
                        seen.push(h);
                        files.push(c.id);
                    }
                }
                Err(_) => files.push(c.id),
            }
        }
        if files.len() >= 2 {
            groups.push(DupGroup { size, hash, files });
        }
    }
    groups.sort_unstable_by_key(|g| std::cmp::Reverse(g.size * (g.files.len() as u64 - 1)));

    Ok(DedupResult {
        groups,
        hashed_bytes,
        cache_hits,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

/* ---------------- commands ---------------- */

const STAGES: [&str; 3] = ["collecting", "prehashing", "hashing"];

#[tauri::command]
pub fn start_dedup(
    app: AppHandle,
    min_size: u64,
    on_event: Channel<DedupEvent>,
) -> Result<(), String> {
    let state = app.state::<DedupState>();
    if state.running.swap(true, Ordering::SeqCst) {
        return Err("Duplicate scan already running".into());
    }
    state.cancel.store(false, Ordering::SeqCst);
    let cancel = state.cancel.clone();

    std::thread::spawn(move || {
        let stage = Arc::new(AtomicU8::new(0));
        let done = Arc::new(AtomicU64::new(0));
        let total = Arc::new(AtomicU64::new(0));
        let finished = Arc::new(AtomicBool::new(false));

        let ticker = {
            let (stage, done, total, finished) =
                (stage.clone(), done.clone(), total.clone(), finished.clone());
            let ch = on_event.clone();
            std::thread::spawn(move || {
                while !finished.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(120));
                    let _ = ch.send(DedupEvent::Progress {
                        stage: STAGES[(stage.load(Ordering::Relaxed) as usize).min(2)],
                        done: done.load(Ordering::Relaxed),
                        total: total.load(Ordering::Relaxed),
                    });
                }
            })
        };

        let result = run_pipeline(&app, min_size, &cancel, &stage, &done, &total);
        finished.store(true, Ordering::Relaxed);
        let _ = ticker.join();

        let state = app.state::<DedupState>();
        state.running.store(false, Ordering::SeqCst);
        match result {
            Ok(res) => {
                let groups = res.groups.len() as u64;
                let wasted: u64 = res
                    .groups
                    .iter()
                    .map(|g| g.size * (g.files.len() as u64 - 1))
                    .sum();
                let event = DedupEvent::Done {
                    groups,
                    wasted_bytes: wasted,
                    hashed_bytes: res.hashed_bytes,
                    cache_hits: res.cache_hits,
                    elapsed_ms: res.elapsed_ms,
                };
                *state.result.write() = Some(res);
                let _ = on_event.send(event);
            }
            Err(message) => {
                let _ = on_event.send(DedupEvent::Error { message });
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn cancel_dedup(state: State<'_, DedupState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupFileDto {
    pub id: u32,
    pub name: String,
    pub dir: String,
    pub mtime: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupGroupDto {
    pub size: u64,
    pub wasted: u64,
    pub hash: String,
    pub files: Vec<DupFileDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DedupResultsDto {
    pub total_groups: u64,
    pub total_wasted: u64,
    pub hashed_bytes: u64,
    pub cache_hits: u64,
    pub elapsed_ms: u64,
    pub groups: Vec<DupGroupDto>,
}

#[tauri::command]
pub fn dedup_results(
    scan: State<'_, ScanState>,
    state: State<'_, DedupState>,
    offset: u32,
    limit: u32,
) -> Result<DedupResultsDto, String> {
    let rguard = state.result.read();
    let res = rguard.as_ref().ok_or("No duplicate scan available")?;
    let tguard = scan.tree.read();
    let tree = tguard.as_ref().ok_or("No scan available")?;

    let mut total_groups = 0u64;
    let mut total_wasted = 0u64;
    let mut page: Vec<DupGroupDto> = Vec::new();
    let limit = limit.clamp(1, 200) as usize;
    let offset = offset as usize;

    for g in &res.groups {
        let live: Vec<u32> = g
            .files
            .iter()
            .copied()
            .filter(|&id| tree.nodes[id as usize].flags & FLAG_DELETED == 0)
            .collect();
        if live.len() < 2 {
            continue;
        }
        let idx = total_groups as usize;
        total_groups += 1;
        total_wasted += g.size * (live.len() as u64 - 1);
        if idx < offset || page.len() >= limit {
            continue;
        }
        page.push(DupGroupDto {
            size: g.size,
            wasted: g.size * (live.len() as u64 - 1),
            hash: g.hash[..6].iter().map(|b| format!("{b:02x}")).collect(),
            files: live
                .into_iter()
                .map(|id| {
                    let n = &tree.nodes[id as usize];
                    DupFileDto {
                        id,
                        name: n.name.to_string(),
                        dir: tree.path_of(n.parent).display().to_string(),
                        mtime: n.mtime,
                    }
                })
                .collect(),
        });
    }

    Ok(DedupResultsDto {
        total_groups,
        total_wasted,
        hashed_bytes: res.hashed_bytes,
        cache_hits: res.cache_hits,
        elapsed_ms: res.elapsed_ms,
        groups: page,
    })
}
