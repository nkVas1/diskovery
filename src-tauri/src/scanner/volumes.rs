use serde::Serialize;
use sysinfo::{DiskKind, Disks};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeInfo {
    pub path: String,
    pub label: String,
    pub fs: String,
    pub total: u64,
    pub free: u64,
    pub kind: String,
    pub removable: bool,
}

pub fn list() -> Vec<VolumeInfo> {
    let disks = Disks::new_with_refreshed_list();
    let mut vols: Vec<VolumeInfo> = disks
        .iter()
        .map(|d| VolumeInfo {
            path: d.mount_point().display().to_string(),
            label: d.name().to_string_lossy().into_owned(),
            fs: d.file_system().to_string_lossy().into_owned(),
            total: d.total_space(),
            free: d.available_space(),
            kind: match d.kind() {
                DiskKind::SSD => "ssd",
                DiskKind::HDD => "hdd",
                _ => "unknown",
            }
            .into(),
            removable: d.is_removable(),
        })
        .collect();
    vols.sort_by(|a, b| a.path.cmp(&b.path));
    vols
}
