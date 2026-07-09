//! Squarified treemap layout + Van Wijk cushion rasterization.
//!
//! The whole frame is computed in the core: layout → per-pixel cushion
//! shading → raw RGBA buffer shipped to the canvas in one IPC response.

use crate::ipc::ScanState;
use crate::scanner::{ScanTree, FLAG_DELETED};
use parking_lot::RwLock;
use serde::Serialize;
use std::cmp::Reverse;
use tauri::ipc::Response;
use tauri::State;

/* ---------------- categories ---------------- */

pub const CAT_VIDEO: u8 = 0;
pub const CAT_IMAGE: u8 = 1;
pub const CAT_DOCUMENT: u8 = 2;
pub const CAT_CODE: u8 = 3;
pub const CAT_OTHER: u8 = 4;
pub const CAT_EXECUTABLE: u8 = 5;
pub const CAT_AUDIO: u8 = 6;
pub const CAT_ARCHIVE: u8 = 7;
pub const CAT_DIR: u8 = 8;

/// Fixed-order categorical palette (dataviz reference, dark mode) + neutral
/// directory fill. Order is the CVD-safety mechanism — do not shuffle.
#[allow(clippy::approx_constant)] // these are colors, not math constants
const COLORS: [[f32; 3]; 9] = [
    [0.224, 0.529, 0.898], // video      — blue    #3987e5
    [0.098, 0.620, 0.439], // image      — aqua    #199e70
    [0.788, 0.522, 0.000], // document   — yellow  #c98500
    [0.000, 0.514, 0.000], // code       — green   #008300
    [0.565, 0.522, 0.914], // other      — violet  #9085e9
    [0.902, 0.404, 0.404], // executable — red     #e66767
    [0.835, 0.318, 0.506], // audio      — magenta #d55181
    [0.851, 0.349, 0.149], // archive    — orange  #d95926
    [0.318, 0.365, 0.443], // directory  — neutral slate
];

pub fn category_of(name: &str) -> u8 {
    let Some((_, ext)) = name.rsplit_once('.') else {
        return CAT_OTHER;
    };
    let ext = ext.to_ascii_lowercase();
    match ext.as_str() {
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg" | "ts"
        | "m2ts" | "vob" => CAT_VIDEO,
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "tif" | "tiff" | "svg" | "ico"
        | "heic" | "raw" | "cr2" | "nef" | "psd" | "ai" => CAT_IMAGE,
        "pdf" | "docx" | "doc" | "xlsx" | "xls" | "pptx" | "ppt" | "txt" | "rtf" | "odt"
        | "csv" | "epub" | "md" => CAT_DOCUMENT,
        "rs" | "tsx" | "jsx" | "js" | "mjs" | "py" | "c" | "cpp" | "h" | "hpp" | "cs" | "java"
        | "kt" | "go" | "rb" | "php" | "html" | "css" | "scss" | "json" | "xml" | "yaml"
        | "yml" | "toml" | "sql" | "sh" | "ps1" | "lock" | "map" => CAT_CODE,
        "exe" | "dll" | "msi" | "sys" | "bat" | "cmd" | "com" | "scr" | "drv" | "ocx" | "cpl" => {
            CAT_EXECUTABLE
        }
        "mp3" | "flac" | "wav" | "aac" | "ogg" | "m4a" | "wma" | "opus" | "mid" | "aiff" => {
            CAT_AUDIO
        }
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "zst" | "iso" | "cab" | "img"
        | "dmg" | "wim" => CAT_ARCHIVE,
        _ => CAT_OTHER,
    }
}

/* ---------------- layout ---------------- */

#[derive(Clone, Copy)]
struct Coef {
    ax: f32,
    bx: f32,
    ay: f32,
    by: f32,
}

struct LeafRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    node: u32,
    coef: Coef,
    color: [f32; 3],
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LabelRect {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub struct Layout {
    pub root: u32,
    pub w: u32,
    pub h: u32,
    leaves: Vec<LeafRect>,
    grid: Vec<Vec<u32>>,
    gcols: u32,
    labels: Vec<LabelRect>,
}

#[derive(Default)]
pub struct TreemapState {
    pub layout: RwLock<Option<Layout>>,
}

const RIDGE_HEIGHT: f32 = 0.55;
const RIDGE_FALLOFF: f32 = 0.72;
const CELL: f32 = 48.0;
const MIN_DESCEND_AREA: f32 = 20.0;
const LABEL_MIN_AREA: f32 = 2600.0;

fn add_ridge(c: &mut Coef, x0: f32, x1: f32, y0: f32, y1: f32, h: f32) {
    let wx = x1 - x0;
    if wx > 0.0 {
        c.ax += -8.0 * h / (wx * wx);
        c.bx += 4.0 * h * (x0 + x1) / (wx * wx);
    }
    let wy = y1 - y0;
    if wy > 0.0 {
        c.ay += -8.0 * h / (wy * wy);
        c.by += 4.0 * h * (y0 + y1) / (wy * wy);
    }
}

/// Squarified layout (Bruls et al.) of `items` (id, size, sorted desc) into
/// the given rect; calls `place` for each item with its computed rect.
fn squarify(
    items: &[(u32, f64)],
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    place: &mut impl FnMut(u32, f64, f64, f64, f64),
) {
    let total: f64 = items.iter().map(|i| i.1).sum();
    if total <= 0.0 || w <= 0.0 || h <= 0.0 {
        return;
    }
    let scale = w * h / total;
    let (mut x, mut y, mut w, mut h) = (x, y, w, h);
    let mut i = 0usize;

    while i < items.len() {
        let side = w.min(h);
        // grow the row while the worst aspect ratio improves
        let mut row_sum = items[i].1 * scale;
        let mut row_end = i + 1;
        let mut worst = worst_ratio(&items[i..row_end], row_sum, side, scale);
        while row_end < items.len() {
            let next_sum = row_sum + items[row_end].1 * scale;
            let next_worst = worst_ratio(&items[i..row_end + 1], next_sum, side, scale);
            if next_worst > worst {
                break;
            }
            row_sum = next_sum;
            worst = next_worst;
            row_end += 1;
        }
        // fix the row along the shorter side
        let thickness = row_sum / side;
        if w <= h {
            // row along the top
            let mut cx = x;
            for &(id, sz) in &items[i..row_end] {
                let iw = if row_sum > 0.0 {
                    sz * scale / thickness
                } else {
                    0.0
                };
                place(id, cx, y, iw, thickness);
                cx += iw;
            }
            y += thickness;
            h -= thickness;
        } else {
            let mut cy = y;
            for &(id, sz) in &items[i..row_end] {
                let ih = if row_sum > 0.0 {
                    sz * scale / thickness
                } else {
                    0.0
                };
                place(id, x, cy, thickness, ih);
                cy += ih;
            }
            x += thickness;
            w -= thickness;
        }
        i = row_end;
    }
}

fn worst_ratio(row: &[(u32, f64)], row_sum: f64, side: f64, scale: f64) -> f64 {
    let thickness = row_sum / side;
    if thickness <= 0.0 {
        return f64::MAX;
    }
    let mut worst: f64 = 1.0;
    for &(_, sz) in row {
        let len = sz * scale / thickness;
        if len <= 0.0 {
            continue;
        }
        let r = (thickness / len).max(len / thickness);
        worst = worst.max(r);
    }
    worst
}

#[allow(clippy::too_many_arguments)] // recursive geometry plumbing
fn layout_node(
    tree: &ScanTree,
    id: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    depth: u32,
    parent_coef: Coef,
    leaves: &mut Vec<LeafRect>,
    labels: &mut Vec<LabelRect>,
) {
    if w < 0.5 || h < 0.5 {
        return;
    }
    let node = &tree.nodes[id as usize];
    let mut coef = parent_coef;
    add_ridge(
        &mut coef,
        x,
        x + w,
        y,
        y + h,
        RIDGE_HEIGHT * RIDGE_FALLOFF.powi(depth as i32),
    );

    let descend = node.is_dir() && node.child_count > 0 && w * h >= MIN_DESCEND_AREA && depth < 40;
    if !descend {
        let color = if node.is_dir() {
            COLORS[CAT_DIR as usize]
        } else {
            COLORS[category_of(&node.name) as usize]
        };
        leaves.push(LeafRect {
            x,
            y,
            w,
            h,
            node: id,
            coef,
            color,
        });
        return;
    }

    let mut items: Vec<(u32, f64)> = (node.child_start..node.child_start + node.child_count)
        .filter(|&i| {
            let c = &tree.nodes[i as usize];
            c.size > 0 && c.flags & FLAG_DELETED == 0
        })
        .map(|i| (i, tree.nodes[i as usize].size as f64))
        .collect();
    if items.is_empty() {
        leaves.push(LeafRect {
            x,
            y,
            w,
            h,
            node: id,
            coef,
            color: COLORS[CAT_DIR as usize],
        });
        return;
    }
    items.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut placements: Vec<(u32, f64, f64, f64, f64)> = Vec::with_capacity(items.len());
    squarify(
        &items,
        x as f64,
        y as f64,
        w as f64,
        h as f64,
        &mut |cid, cx, cy, cw, ch| {
            placements.push((cid, cx, cy, cw, ch));
        },
    );

    for (cid, cx, cy, cw, ch) in placements {
        let (cx, cy, cw, ch) = (cx as f32, cy as f32, cw as f32, ch as f32);
        let child = &tree.nodes[cid as usize];
        if depth == 0 && child.is_dir() && cw * ch >= LABEL_MIN_AREA {
            labels.push(LabelRect {
                id: cid,
                name: child.name.to_string(),
                x: cx,
                y: cy,
                w: cw,
                h: ch,
            });
        }
        layout_node(tree, cid, cx, cy, cw, ch, depth + 1, coef, leaves, labels);
    }
}

/* ---------------- rasterization ---------------- */

const LIGHT: [f32; 3] = [-0.437, -0.437, 0.786]; // normalized (-0.5,-0.5,0.9)
const AMBIENT: f32 = 0.40;
const DIFFUSE: f32 = 0.62;

fn rasterize(layout: &Layout) -> Vec<u8> {
    let (w, h) = (layout.w as usize, layout.h as usize);
    let mut buf = vec![0u8; w * h * 4];
    // background = void
    for px in buf.chunks_exact_mut(4) {
        px[0] = 6;
        px[1] = 9;
        px[2] = 17;
        px[3] = 255;
    }
    for leaf in &layout.leaves {
        let x0 = leaf.x.max(0.0) as usize;
        let y0 = leaf.y.max(0.0) as usize;
        let x1 = ((leaf.x + leaf.w).ceil() as usize).min(w);
        let y1 = ((leaf.y + leaf.h).ceil() as usize).min(h);
        let c = leaf.coef;
        for py in y0..y1 {
            let fy = py as f32 + 0.5;
            let ny = -(c.ay * fy + c.by);
            let row = &mut buf[(py * w + x0) * 4..(py * w + x1) * 4];
            let mut fx = x0 as f32 + 0.5;
            for px in row.chunks_exact_mut(4) {
                let nx = -(c.ax * fx + c.bx);
                let inv_len = 1.0 / (nx * nx + ny * ny + 1.0).sqrt();
                let dot = (nx * LIGHT[0] + ny * LIGHT[1] + LIGHT[2]) * inv_len;
                let i = AMBIENT + DIFFUSE * dot.max(0.0);
                px[0] = (leaf.color[0] * i * 255.0) as u8;
                px[1] = (leaf.color[1] * i * 255.0) as u8;
                px[2] = (leaf.color[2] * i * 255.0) as u8;
                fx += 1.0;
            }
        }
    }
    buf
}

fn build_grid(leaves: &[LeafRect], w: u32, h: u32) -> (Vec<Vec<u32>>, u32) {
    let gcols = (w as f32 / CELL).ceil().max(1.0) as u32;
    let grows = (h as f32 / CELL).ceil().max(1.0) as u32;
    let mut grid = vec![Vec::new(); (gcols * grows) as usize];
    for (i, leaf) in leaves.iter().enumerate() {
        let cx0 = (leaf.x / CELL) as u32;
        let cy0 = (leaf.y / CELL) as u32;
        let cx1 = (((leaf.x + leaf.w) / CELL) as u32).min(gcols - 1);
        let cy1 = (((leaf.y + leaf.h) / CELL) as u32).min(grows - 1);
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                grid[(cy * gcols + cx) as usize].push(i as u32);
            }
        }
    }
    (grid, gcols)
}

pub fn build(tree: &ScanTree, root: u32, w: u32, h: u32) -> Layout {
    let mut leaves = Vec::new();
    let mut labels = Vec::new();
    layout_node(
        tree,
        root,
        0.0,
        0.0,
        w as f32,
        h as f32,
        0,
        Coef {
            ax: 0.0,
            bx: 0.0,
            ay: 0.0,
            by: 0.0,
        },
        &mut leaves,
        &mut labels,
    );
    labels.sort_unstable_by_key(|l| Reverse((l.w * l.h) as u64));
    labels.truncate(24);
    let (grid, gcols) = build_grid(&leaves, w, h);
    Layout {
        root,
        w,
        h,
        leaves,
        grid,
        gcols,
        labels,
    }
}

/* ---------------- commands ---------------- */

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Crumb {
    pub id: u32,
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreemapMeta {
    pub root: u32,
    pub labels: Vec<LabelRect>,
    pub breadcrumb: Vec<Crumb>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub id: u32,
    pub top_dir: u32,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub category: u8,
    pub rect: [f32; 4],
}

#[tauri::command]
pub fn treemap_render(
    scan: State<'_, ScanState>,
    tm: State<'_, TreemapState>,
    root: u32,
    width: u32,
    height: u32,
) -> Result<Response, String> {
    let guard = scan.tree.read();
    let tree = guard.as_ref().ok_or("No scan available")?;
    if root as usize >= tree.nodes.len() {
        return Err("Unknown node".into());
    }
    let width = width.clamp(64, 8192);
    let height = height.clamp(64, 8192);
    let layout = build(tree, root, width, height);
    let pixels = rasterize(&layout);
    *tm.layout.write() = Some(layout);
    Ok(Response::new(pixels))
}

#[tauri::command]
pub fn treemap_meta(
    scan: State<'_, ScanState>,
    tm: State<'_, TreemapState>,
) -> Result<TreemapMeta, String> {
    let guard = scan.tree.read();
    let tree = guard.as_ref().ok_or("No scan available")?;
    let lg = tm.layout.read();
    let layout = lg.as_ref().ok_or("No layout")?;

    let mut breadcrumb = Vec::new();
    let mut cur = layout.root;
    loop {
        breadcrumb.push(Crumb {
            id: cur,
            name: tree.nodes[cur as usize].name.to_string(),
        });
        if cur == 0 {
            break;
        }
        cur = tree.nodes[cur as usize].parent;
    }
    breadcrumb.reverse();

    Ok(TreemapMeta {
        root: layout.root,
        labels: layout.labels.clone(),
        breadcrumb,
    })
}

#[tauri::command]
pub fn treemap_hit(
    scan: State<'_, ScanState>,
    tm: State<'_, TreemapState>,
    x: f32,
    y: f32,
) -> Result<Option<Hit>, String> {
    let guard = scan.tree.read();
    let tree = guard.as_ref().ok_or("No scan available")?;
    let lg = tm.layout.read();
    let layout = lg.as_ref().ok_or("No layout")?;

    let cx = (x / CELL) as u32;
    let cy = (y / CELL) as u32;
    if cx >= layout.gcols {
        return Ok(None);
    }
    let cell = match layout.grid.get((cy * layout.gcols + cx) as usize) {
        Some(c) => c,
        None => return Ok(None),
    };
    for &li in cell {
        let leaf = &layout.leaves[li as usize];
        if x >= leaf.x && x < leaf.x + leaf.w && y >= leaf.y && y < leaf.y + leaf.h {
            let node = &tree.nodes[leaf.node as usize];
            // ancestor that is a direct child of the current layout root
            let mut top = leaf.node;
            while top != layout.root && tree.nodes[top as usize].parent != layout.root {
                top = tree.nodes[top as usize].parent;
            }
            return Ok(Some(Hit {
                id: leaf.node,
                top_dir: top,
                name: node.name.to_string(),
                path: tree.path_of(leaf.node).display().to_string(),
                size: node.size,
                is_dir: node.is_dir(),
                category: if node.is_dir() {
                    CAT_DIR
                } else {
                    category_of(&node.name)
                },
                rect: [leaf.x, leaf.y, leaf.w, leaf.h],
            }));
        }
    }
    Ok(None)
}
