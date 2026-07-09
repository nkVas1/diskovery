//! Anonymized statistical digest of a scan — the ONLY structure the AI ever
//! receives. User-created folder names are replaced by tokens; the
//! token → real-path mapping never leaves the machine.

use crate::advisor;
use crate::dedup::DedupState;
use crate::scanner::{ScanTree, FLAG_DELETED, FLAG_DIR, FLAG_REPARSE};
use crate::treemap::category_of;
use serde::Serialize;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const CATEGORY_NAMES: [&str; 8] = [
    "video",
    "images",
    "documents",
    "code",
    "other",
    "executables",
    "audio",
    "archives",
];

/// Folder names that are meaningful to analysis and safe to send verbatim.
const WELL_KNOWN: &[&str] = &[
    "windows",
    "program files",
    "program files (x86)",
    "programdata",
    "users",
    "appdata",
    "local",
    "locallow",
    "roaming",
    "documents",
    "downloads",
    "desktop",
    "pictures",
    "videos",
    "music",
    "onedrive",
    "node_modules",
    "target",
    "src",
    "dist",
    "build",
    ".cargo",
    ".gradle",
    ".nuget",
    ".vscode",
    ".android",
    "steam",
    "steamapps",
    "common",
    "games",
    "epic games",
    "temp",
    "tmp",
    "cache",
    "caches",
    "logs",
    "backup",
    "backups",
    "projects",
    "repos",
    "github",
    "coding",
    "dev",
    "work",
    "google",
    "microsoft",
    "nvidia",
    "amd",
    "intel",
    "docker",
    "wsl",
    "packages",
    "installer",
    "softwaredistribution",
    "winsxs",
    "system32",
    "syswow64",
    "$recycle.bin",
    "windows.old",
    "virtualbox vms",
    "vms",
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryStat {
    name: &'static str,
    bytes: u64,
    files: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtStatOut {
    ext: String,
    bytes: u64,
    count: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderStat {
    path: String,
    bytes: u64,
    share_pct: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeFile {
    name: String,
    bytes: u64,
    age_days: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgeProfile {
    bytes_older_1y: u64,
    bytes_older_2y: u64,
    bytes_older_5y: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupStat {
    groups: u64,
    wasted_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvisorStat {
    rule_id: String,
    tier: &'static str,
    bytes: u64,
    deletable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Digest {
    scan_root: String,
    total_bytes: u64,
    files: u64,
    dirs: u64,
    categories: Vec<CategoryStat>,
    extensions: Vec<ExtStatOut>,
    top_folders: Vec<FolderStat>,
    age_profile: AgeProfile,
    large_files: Vec<LargeFile>,
    duplicates: Option<DupStat>,
    advisor_findings: Vec<AdvisorStat>,
    os: String,
}

pub struct DigestBundle {
    pub digest: Digest,
    /// token (e.g. "<dir3>") → real absolute path; stays on-device.
    pub token_map: HashMap<String, String>,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

struct Sanitizer {
    username: Option<String>,
    counter: u32,
    map: HashMap<String, String>,
}

impl Sanitizer {
    fn new() -> Self {
        Self {
            username: std::env::var("USERNAME").ok().filter(|s| !s.is_empty()),
            counter: 0,
            map: HashMap::new(),
        }
    }

    fn sanitize_path(&mut self, tree: &ScanTree, id: u32) -> String {
        let real = tree.path_of(id).display().to_string();
        let mut parts: Vec<String> = Vec::new();
        for (i, seg) in real.split('\\').enumerate() {
            if i == 0 {
                parts.push(seg.to_string()); // drive letter
                continue;
            }
            let lower = seg.to_ascii_lowercase();
            if WELL_KNOWN.contains(&lower.as_str()) {
                parts.push(seg.to_string());
            } else if self
                .username
                .as_deref()
                .is_some_and(|u| u.eq_ignore_ascii_case(seg))
            {
                parts.push("<user>".into());
            } else {
                self.counter += 1;
                let token = format!("<dir{}>", self.counter);
                self.map.insert(token.clone(), real.clone());
                parts.push(token);
            }
        }
        parts.join("\\")
    }

    fn sanitize_file_name(&mut self, tree: &ScanTree, id: u32) -> String {
        let n = &tree.nodes[id as usize];
        let ext = n.name.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
        self.counter += 1;
        let token = format!("<file{}>", self.counter);
        self.map
            .insert(token.clone(), tree.path_of(id).display().to_string());
        if ext.is_empty() {
            token
        } else {
            format!("{token}.{ext}")
        }
    }
}

fn dir_depth(tree: &ScanTree, id: u32) -> u32 {
    let mut d = 0;
    let mut cur = id;
    while cur != 0 {
        cur = tree.nodes[cur as usize].parent;
        d += 1;
    }
    d
}

pub fn build(tree: &ScanTree, dedup: &DedupState) -> DigestBundle {
    let now = now_secs();
    let mut san = Sanitizer::new();

    // categories + age profile in one pass over files
    let mut cat_bytes = [0u64; 8];
    let mut cat_files = [0u64; 8];
    let mut age = AgeProfile {
        bytes_older_1y: 0,
        bytes_older_2y: 0,
        bytes_older_5y: 0,
    };
    for n in &tree.nodes {
        if n.flags & (FLAG_DIR | FLAG_REPARSE | FLAG_DELETED) != 0 || n.size == 0 {
            continue;
        }
        let c = category_of(&n.name) as usize;
        if c < 8 {
            cat_bytes[c] += n.size;
            cat_files[c] += 1;
        }
        if n.mtime > 0 {
            let age_s = now - n.mtime;
            if age_s > 365 * 86_400 {
                age.bytes_older_1y += n.size;
            }
            if age_s > 2 * 365 * 86_400 {
                age.bytes_older_2y += n.size;
            }
            if age_s > 5 * 365 * 86_400 {
                age.bytes_older_5y += n.size;
            }
        }
    }
    let mut categories: Vec<CategoryStat> = (0..8)
        .map(|i| CategoryStat {
            name: CATEGORY_NAMES[i],
            bytes: cat_bytes[i],
            files: cat_files[i],
        })
        .filter(|c| c.bytes > 0)
        .collect();
    categories.sort_unstable_by_key(|c| std::cmp::Reverse(c.bytes));

    // top folders: dirs at depth ≤ 3 holding ≥ 1.5% of total
    let threshold = tree.bytes / 66;
    let mut folders: Vec<(u32, u64)> = tree
        .nodes
        .iter()
        .enumerate()
        .filter(|(i, n)| {
            *i != 0
                && n.is_dir()
                && n.flags & FLAG_DELETED == 0
                && n.size >= threshold.max(1)
                && dir_depth(tree, *i as u32) <= 3
        })
        .map(|(i, n)| (i as u32, n.size))
        .collect();
    folders.sort_unstable_by_key(|&(_, s)| std::cmp::Reverse(s));
    folders.truncate(12);
    let top_folders: Vec<FolderStat> = folders
        .into_iter()
        .map(|(id, bytes)| FolderStat {
            path: san.sanitize_path(tree, id),
            bytes,
            share_pct: if tree.bytes > 0 {
                (bytes as f64 / tree.bytes as f64 * 100.0) as f32
            } else {
                0.0
            },
        })
        .collect();

    let large_files: Vec<LargeFile> = tree
        .top_files
        .iter()
        .take(10)
        .map(|&id| {
            let n = &tree.nodes[id as usize];
            LargeFile {
                name: san.sanitize_file_name(tree, id),
                bytes: n.size,
                age_days: if n.mtime > 0 {
                    (now - n.mtime) / 86_400
                } else {
                    -1
                },
            }
        })
        .collect();

    let duplicates = dedup.result.read().as_ref().map(|r| DupStat {
        groups: r.groups.len() as u64,
        wasted_bytes: r
            .groups
            .iter()
            .map(|g| g.size * (g.files.len() as u64 - 1))
            .sum(),
    });

    let advisor_findings: Vec<AdvisorStat> = advisor::evaluate(tree)
        .iter()
        .map(|f| AdvisorStat {
            rule_id: f.rule.id.to_string(),
            tier: match f.rule.tier {
                advisor::rules::Tier::Safe => "safe",
                advisor::rules::Tier::Caution => "caution",
                advisor::rules::Tier::Expert => "expert",
            },
            bytes: f.bytes,
            deletable: f.rule.deletable,
        })
        .collect();

    // scan root: drive roots are not sensitive; deeper roots get sanitized
    let root_display = tree.root_path.display().to_string();
    let scan_root = if root_display.len() <= 3 {
        root_display
    } else {
        san.sanitize_path(tree, 0)
    };

    let digest = Digest {
        scan_root,
        total_bytes: tree.bytes,
        files: tree.files,
        dirs: tree.dirs,
        categories,
        extensions: tree
            .ext_stats
            .iter()
            .take(15)
            .map(|e| ExtStatOut {
                ext: e.ext.clone(),
                bytes: e.bytes,
                count: e.count,
            })
            .collect(),
        top_folders,
        age_profile: age,
        large_files,
        duplicates,
        advisor_findings,
        os: "Windows 11".into(),
    };

    DigestBundle {
        digest,
        token_map: san.map,
    }
}
