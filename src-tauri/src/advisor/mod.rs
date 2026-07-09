//! Rules engine: evaluates the knowledge base against the scan tree and
//! executes tier-gated cleanups (always via the Recycle Bin).

mod rules;

use crate::ipc::ScanState;
use crate::scanner::{ScanTree, FLAG_DELETED, FLAG_DIR, FLAG_REPARSE};
use parking_lot::RwLock;
use rules::{Matcher, Rule, Tier, RULES};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

pub struct Finding {
    pub rule: &'static Rule,
    pub bytes: u64,
    pub items: Vec<u32>,
    pub locations: Vec<String>,
}

#[derive(Default)]
pub struct AdvisorState {
    pub findings: RwLock<Option<Vec<Finding>>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingDto {
    pub rule_id: String,
    pub title: String,
    pub tier: Tier,
    pub rationale: String,
    pub action_hint: String,
    pub bytes: u64,
    pub item_count: u32,
    pub locations: Vec<String>,
    pub deletable: bool,
}

/* ---------------- matching helpers ---------------- */

fn expand_env(template: &str) -> Option<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let end = after.find('%')?;
        let var = &after[..end];
        out.push_str(&std::env::var(var).ok()?);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// Locate `abs_path` inside the tree (case-insensitive), if the scanned root
/// is a prefix of it. Returns the node id.
fn find_node(tree: &ScanTree, abs_path: &str) -> Option<u32> {
    let root = tree.root_path.to_string_lossy();
    let root = root.trim_end_matches('\\');
    let abs = abs_path.trim_end_matches('\\');
    if abs.len() < root.len() || !abs[..root.len()].eq_ignore_ascii_case(root) {
        return None;
    }
    let rest = &abs[root.len()..];
    let rest = rest.trim_start_matches('\\');
    let mut cur = 0u32;
    if rest.is_empty() {
        return Some(cur);
    }
    'comp: for comp in rest.split('\\') {
        let n = &tree.nodes[cur as usize];
        for i in n.child_start..n.child_start + n.child_count {
            if tree.nodes[i as usize].name.eq_ignore_ascii_case(comp) {
                cur = i;
                continue 'comp;
            }
        }
        return None;
    }
    Some(cur)
}

fn has_ancestor_in(tree: &ScanTree, id: u32, set: &[u32]) -> bool {
    let mut cur = id;
    while cur != 0 {
        cur = tree.nodes[cur as usize].parent;
        if set.contains(&cur) {
            return true;
        }
    }
    false
}

fn dir_has_child_named(tree: &ScanTree, dir: u32, name: &str) -> bool {
    let n = &tree.nodes[dir as usize];
    (n.child_start..n.child_start + n.child_count)
        .any(|i| tree.nodes[i as usize].name.eq_ignore_ascii_case(name))
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn evaluate(tree: &ScanTree) -> Vec<Finding> {
    let now = now_secs();
    let mut findings = Vec::new();

    for rule in RULES {
        let mut items: Vec<u32> = Vec::new();

        match &rule.matcher {
            Matcher::AbsPath(templates) => {
                for t in *templates {
                    if let Some(p) = expand_env(t) {
                        if let Some(id) = find_node(tree, &p) {
                            let n = &tree.nodes[id as usize];
                            if n.flags & FLAG_DELETED == 0 && n.size > 0 {
                                items.push(id);
                            }
                        }
                    }
                }
            }
            Matcher::RootRel(rels) => {
                let root = tree.root_path.to_string_lossy();
                let drive: String = root.chars().take_while(|c| *c != '\\').collect();
                for rel in *rels {
                    let p = format!("{drive}\\{rel}");
                    if let Some(id) = find_node(tree, &p) {
                        let n = &tree.nodes[id as usize];
                        if n.flags & FLAG_DELETED == 0 && n.size > 0 {
                            items.push(id);
                        }
                    }
                }
            }
            Matcher::DirName {
                names,
                sibling,
                stale_days,
            } => {
                for (i, n) in tree.nodes.iter().enumerate() {
                    if !n.is_dir() || n.flags & FLAG_DELETED != 0 || n.size == 0 {
                        continue;
                    }
                    if !names.iter().any(|m| n.name.eq_ignore_ascii_case(m)) {
                        continue;
                    }
                    if let Some(days) = stale_days {
                        if n.mtime == 0 || now - n.mtime < i64::from(*days) * 86_400 {
                            continue;
                        }
                    }
                    if let Some(sib) = sibling {
                        if !dir_has_child_named(tree, n.parent, sib) {
                            continue;
                        }
                    }
                    let id = i as u32;
                    if !has_ancestor_in(tree, id, &items) {
                        items.push(id);
                    }
                }
            }
            Matcher::FileExt {
                exts,
                min_file_size,
            } => {
                for (i, n) in tree.nodes.iter().enumerate() {
                    if n.flags & (FLAG_DIR | FLAG_REPARSE | FLAG_DELETED) != 0
                        || n.size < *min_file_size
                    {
                        continue;
                    }
                    let matched = n
                        .name
                        .rsplit_once('.')
                        .map(|(_, e)| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
                        .unwrap_or(false);
                    if matched {
                        items.push(i as u32);
                    }
                }
            }
        }

        if items.is_empty() {
            continue;
        }
        let bytes: u64 = items.iter().map(|&id| tree.nodes[id as usize].size).sum();
        if bytes == 0 {
            continue;
        }
        let mut by_size = items.clone();
        by_size.sort_unstable_by_key(|&id| std::cmp::Reverse(tree.nodes[id as usize].size));
        let locations = by_size
            .iter()
            .take(5)
            .map(|&id| tree.path_of(id).display().to_string())
            .collect();
        findings.push(Finding {
            rule,
            bytes,
            items,
            locations,
        });
    }

    findings.sort_by_key(|f| {
        let tier_order = match f.rule.tier {
            Tier::Safe => 0u8,
            Tier::Caution => 1,
            Tier::Expert => 2,
        };
        (tier_order, std::cmp::Reverse(f.bytes))
    });
    findings
}

/* ---------------- commands ---------------- */

#[tauri::command]
pub fn advisor_analyze(
    scan: State<'_, ScanState>,
    state: State<'_, AdvisorState>,
) -> Result<Vec<FindingDto>, String> {
    let guard = scan.tree.read();
    let tree = guard.as_ref().ok_or("No scan available")?;
    let findings = evaluate(tree);
    let dtos = findings
        .iter()
        .map(|f| FindingDto {
            rule_id: f.rule.id.to_string(),
            title: f.rule.title.to_string(),
            tier: f.rule.tier,
            rationale: f.rule.rationale.to_string(),
            action_hint: f.rule.action_hint.to_string(),
            bytes: f.bytes,
            item_count: f.items.len() as u32,
            locations: f.locations.clone(),
            deletable: f.rule.deletable,
        })
        .collect();
    *state.findings.write() = Some(findings);
    Ok(dtos)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    pub removed_bytes: u64,
    pub removed_items: u32,
    pub failed_items: u32,
}

#[tauri::command]
pub fn advisor_clean(
    scan: State<'_, ScanState>,
    state: State<'_, AdvisorState>,
    rule_id: String,
) -> Result<CleanReport, String> {
    let items: Vec<u32> = {
        let fguard = state.findings.read();
        let findings = fguard.as_ref().ok_or("Run analyze first")?;
        let f = findings
            .iter()
            .find(|f| f.rule.id == rule_id)
            .ok_or("Unknown finding")?;
        if !f.rule.deletable {
            return Err("This finding is advice-only".into());
        }
        f.items.clone()
    };

    let mut removed_bytes = 0u64;
    let mut removed_items = 0u32;
    let mut failed_items = 0u32;

    let mut guard = scan.tree.write();
    let tree = guard.as_mut().ok_or("No scan available")?;

    for id in items {
        let n = &tree.nodes[id as usize];
        if n.flags & FLAG_DELETED != 0 {
            continue;
        }
        let path = tree.path_of(id);
        match trash::delete(&path) {
            Ok(()) => {
                let size = tree.nodes[id as usize].size;
                tree.nodes[id as usize].size = 0;
                tree.nodes[id as usize].flags |= FLAG_DELETED;
                let mut cur = id;
                while cur != 0 {
                    cur = tree.nodes[cur as usize].parent;
                    let a = &mut tree.nodes[cur as usize];
                    a.size = a.size.saturating_sub(size);
                }
                tree.bytes = tree.bytes.saturating_sub(size);
                removed_bytes += size;
                removed_items += 1;
            }
            Err(_) => failed_items += 1,
        }
    }

    Ok(CleanReport {
        removed_bytes,
        removed_items,
        failed_items,
    })
}
