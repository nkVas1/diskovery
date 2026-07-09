//! The cleanup knowledge base. Every rule carries a safety tier and a
//! human rationale — the advisor never suggests anything it can't explain.

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Safe,
    Caution,
    Expert,
}

pub enum Matcher {
    /// Absolute path templates with %ENV% expansion.
    AbsPath(&'static [&'static str]),
    /// Paths relative to the root of the scanned volume (Windows.old, …).
    RootRel(&'static [&'static str]),
    /// Any directory with one of these names, anywhere in the tree.
    /// `sibling`: require this file next to the dir (e.g. Cargo.toml for target/).
    /// `stale_days`: only match if the dir mtime is older than N days.
    DirName {
        names: &'static [&'static str],
        sibling: Option<&'static str>,
        stale_days: Option<u32>,
    },
    /// Files by extension, with an optional per-file minimum size.
    FileExt {
        exts: &'static [&'static str],
        min_file_size: u64,
    },
}

pub struct Rule {
    pub id: &'static str,
    pub title: &'static str,
    pub tier: Tier,
    pub rationale: &'static str,
    pub action_hint: &'static str,
    pub matcher: Matcher,
    /// false = advice only: the advisor reports it but offers no delete button.
    pub deletable: bool,
}

pub const RULES: &[Rule] = &[
    /* ---------------- SAFE ---------------- */
    Rule {
        id: "windows-temp",
        title: "Windows temp files",
        tier: Tier::Safe,
        rationale: "Scratch space for the OS and installers. Files in use are skipped automatically; everything else is safe to remove.",
        action_hint: "Recycles the contents of the system temp folders.",
        matcher: Matcher::AbsPath(&["%WINDIR%\\Temp", "%TEMP%"]),
        deletable: true,
    },
    Rule {
        id: "browser-caches",
        title: "Browser caches",
        tier: Tier::Safe,
        rationale: "Pages and media cached for faster loading. Browsers rebuild these transparently; you may briefly notice slower first loads.",
        action_hint: "Close browsers first for a complete clean.",
        matcher: Matcher::AbsPath(&[
            "%LOCALAPPDATA%\\Google\\Chrome\\User Data\\Default\\Cache",
            "%LOCALAPPDATA%\\Google\\Chrome\\User Data\\Default\\Code Cache",
            "%LOCALAPPDATA%\\Microsoft\\Edge\\User Data\\Default\\Cache",
            "%LOCALAPPDATA%\\Microsoft\\Edge\\User Data\\Default\\Code Cache",
            "%LOCALAPPDATA%\\BraveSoftware\\Brave-Browser\\User Data\\Default\\Cache",
            "%LOCALAPPDATA%\\Vivaldi\\User Data\\Default\\Cache",
            "%LOCALAPPDATA%\\Opera Software\\Opera Stable\\Cache",
            "%APPDATA%\\Mozilla\\Firefox\\Profiles",
        ]),
        deletable: true,
    },
    Rule {
        id: "shader-caches",
        title: "GPU shader caches",
        tier: Tier::Safe,
        rationale: "Compiled shaders for games and apps. Drivers rebuild them on demand; first launches may stutter briefly.",
        action_hint: "Rebuilt automatically by the GPU driver.",
        matcher: Matcher::AbsPath(&[
            "%LOCALAPPDATA%\\NVIDIA\\DXCache",
            "%LOCALAPPDATA%\\NVIDIA\\GLCache",
            "%LOCALAPPDATA%\\AMD\\DxCache",
            "%LOCALAPPDATA%\\AMD\\DxcCache",
            "%LOCALAPPDATA%\\Intel\\ShaderCache",
            "%LOCALAPPDATA%\\D3DSCache",
        ]),
        deletable: true,
    },
    Rule {
        id: "thumbnail-cache",
        title: "Explorer thumbnail cache",
        tier: Tier::Safe,
        rationale: "Cached previews for folders and images. Explorer regenerates them as you browse.",
        action_hint: "Thumbnails will regenerate on first view.",
        matcher: Matcher::AbsPath(&["%LOCALAPPDATA%\\Microsoft\\Windows\\Explorer"]),
        deletable: true,
    },
    Rule {
        id: "crash-dumps",
        title: "Crash dumps & error reports",
        tier: Tier::Safe,
        rationale: "Memory dumps from crashed apps and Windows Error Reporting queues. Only useful if you are actively debugging a crash.",
        action_hint: "Keep them only if you plan to analyze a recent crash.",
        matcher: Matcher::AbsPath(&[
            "%LOCALAPPDATA%\\CrashDumps",
            "%LOCALAPPDATA%\\Microsoft\\Windows\\WER",
            "%WINDIR%\\Minidump",
            "%WINDIR%\\MEMORY.DMP",
        ]),
        deletable: true,
    },
    Rule {
        id: "package-manager-caches",
        title: "Package manager caches (npm, pip, NuGet, Gradle, Cargo)",
        tier: Tier::Safe,
        rationale: "Downloaded packages kept for offline reinstalls. Removing them costs only re-download bandwidth on the next build.",
        action_hint: "Next builds will re-download dependencies.",
        matcher: Matcher::AbsPath(&[
            "%LOCALAPPDATA%\\npm-cache",
            "%APPDATA%\\npm-cache",
            "%LOCALAPPDATA%\\pip\\cache",
            "%USERPROFILE%\\.nuget\\packages",
            "%USERPROFILE%\\.gradle\\caches",
            "%USERPROFILE%\\.cargo\\registry\\cache",
            "%USERPROFILE%\\.cargo\\registry\\src",
            "%LOCALAPPDATA%\\pnpm\\store",
            "%LOCALAPPDATA%\\Yarn\\Cache",
        ]),
        deletable: true,
    },
    Rule {
        id: "app-caches",
        title: "App caches (Discord, Teams, Slack, Spotify)",
        tier: Tier::Safe,
        rationale: "Chat media and web caches of desktop apps. Rebuilt automatically; close the apps first for a complete clean.",
        action_hint: "Close the apps before cleaning.",
        matcher: Matcher::AbsPath(&[
            "%APPDATA%\\discord\\Cache",
            "%APPDATA%\\discord\\Code Cache",
            "%APPDATA%\\Microsoft\\Teams\\Cache",
            "%APPDATA%\\Slack\\Cache",
            "%LOCALAPPDATA%\\Spotify\\Data",
        ]),
        deletable: true,
    },
    Rule {
        id: "temp-file-ext",
        title: "Scattered *.tmp files",
        tier: Tier::Safe,
        rationale: "Leftover temporary files that applications forgot to remove.",
        action_hint: "Files currently in use are skipped automatically.",
        matcher: Matcher::FileExt {
            exts: &["tmp", "temp"],
            min_file_size: 64 * 1024,
        },
        deletable: true,
    },
    /* ---------------- CAUTION ---------------- */
    Rule {
        id: "windows-update-cache",
        title: "Windows Update download cache",
        tier: Tier::Caution,
        rationale: "Already-installed update packages. Safe when no update is mid-install — cleaning during an active update can corrupt it.",
        action_hint: "Make sure Windows Update is not currently installing.",
        matcher: Matcher::AbsPath(&["%WINDIR%\\SoftwareDistribution\\Download"]),
        deletable: true,
    },
    Rule {
        id: "delivery-optimization",
        title: "Delivery Optimization cache",
        tier: Tier::Caution,
        rationale: "Update chunks shared with other PCs on your network. Windows re-downloads what it needs.",
        action_hint: "Prefer Settings → Storage Sense for this one; manual removal may need admin rights.",
        matcher: Matcher::AbsPath(&[
            "%WINDIR%\\ServiceProfiles\\NetworkService\\AppData\\Local\\Microsoft\\Windows\\DeliveryOptimization\\Cache",
        ]),
        deletable: false,
    },
    Rule {
        id: "windows-old",
        title: "Previous Windows installation (Windows.old)",
        tier: Tier::Caution,
        rationale: "Kept so you can roll back a feature update. Removing it frees a lot of space but makes the rollback impossible.",
        action_hint: "If Windows has been stable for a few weeks, this is a big easy win.",
        matcher: Matcher::RootRel(&["Windows.old"]),
        deletable: true,
    },
    Rule {
        id: "hibernation-file",
        title: "Hibernation file (hiberfil.sys)",
        tier: Tier::Caution,
        rationale: "Reserved for hibernation and Fast Startup — usually 40% of your RAM size. If you never hibernate, it can be disabled.",
        action_hint: "Run `powercfg /h off` in an elevated terminal — the file cannot simply be deleted.",
        matcher: Matcher::RootRel(&["hiberfil.sys"]),
        deletable: false,
    },
    Rule {
        id: "pagefile-info",
        title: "Paging file (pagefile.sys)",
        tier: Tier::Caution,
        rationale: "Virtual memory backing store, managed by Windows. Do not delete; its size can be tuned in System → Advanced settings.",
        action_hint: "Adjust via SystemPropertiesPerformance.exe → Advanced → Virtual memory.",
        matcher: Matcher::RootRel(&["pagefile.sys", "swapfile.sys"]),
        deletable: false,
    },
    Rule {
        id: "recycle-bin",
        title: "Recycle Bin contents",
        tier: Tier::Caution,
        rationale: "Files you already deleted, awaiting final removal. Emptying makes the deletion permanent.",
        action_hint: "Empty via the Recycle Bin icon to keep shell integration intact.",
        matcher: Matcher::RootRel(&["$Recycle.Bin"]),
        deletable: false,
    },
    Rule {
        id: "stale-node-modules",
        title: "node_modules of stale projects",
        tier: Tier::Caution,
        rationale: "Dependency folders untouched for 90+ days. One `npm install` restores any of them — but only remove those of projects you are done with.",
        action_hint: "Restored by `npm install` in the project folder.",
        matcher: Matcher::DirName {
            names: &["node_modules"],
            sibling: Some("package.json"),
            stale_days: Some(90),
        },
        deletable: true,
    },
    Rule {
        id: "stale-rust-targets",
        title: "Rust target/ dirs of stale projects",
        tier: Tier::Caution,
        rationale: "Build artifacts untouched for 60+ days, next to a Cargo.toml. `cargo build` recreates them from scratch.",
        action_hint: "Restored by `cargo build` (takes compile time).",
        matcher: Matcher::DirName {
            names: &["target"],
            sibling: Some("Cargo.toml"),
            stale_days: Some(60),
        },
        deletable: true,
    },
    Rule {
        id: "big-logs",
        title: "Oversized log files",
        tier: Tier::Caution,
        rationale: "Log files above 16 MB. Applications usually rotate or recreate logs, but check anything that looks actively written.",
        action_hint: "Skip logs of services you are currently debugging.",
        matcher: Matcher::FileExt {
            exts: &["log", "etl"],
            min_file_size: 16 * 1024 * 1024,
        },
        deletable: true,
    },
    Rule {
        id: "disc-images",
        title: "Large disc images (ISO/IMG)",
        tier: Tier::Caution,
        rationale: "Installer and disc images above 512 MB. Often kept 'just in case' long after they were burned or mounted.",
        action_hint: "Review each — some may be irreplaceable.",
        matcher: Matcher::FileExt {
            exts: &["iso", "img", "wim"],
            min_file_size: 512 * 1024 * 1024,
        },
        deletable: true,
    },
    Rule {
        id: "docker-wsl",
        title: "Docker / WSL virtual disks",
        tier: Tier::Caution,
        rationale: "ext4.vhdx files grow but never shrink on their own, even after images are deleted inside.",
        action_hint: "Run `docker system prune` then compact the VHDX via `wsl --shutdown` + diskpart, or Docker Desktop → Clean up.",
        matcher: Matcher::AbsPath(&[
            "%LOCALAPPDATA%\\Docker\\wsl",
            "%LOCALAPPDATA%\\Packages\\CanonicalGroupLimited.Ubuntu_79rhkp1fndgsc\\LocalState",
        ]),
        deletable: false,
    },
    /* ---------------- EXPERT ---------------- */
    Rule {
        id: "winsxs-note",
        title: "Windows component store (WinSxS)",
        tier: Tier::Expert,
        rationale: "Holds every Windows component version for servicing. Its apparent size is inflated by hardlinks. Never delete manually.",
        action_hint: "Clean safely with `Dism /Online /Cleanup-Image /StartComponentCleanup`.",
        matcher: Matcher::AbsPath(&["%WINDIR%\\WinSxS"]),
        deletable: false,
    },
    Rule {
        id: "installer-store",
        title: "Windows Installer store",
        tier: Tier::Expert,
        rationale: "MSI/MSP files required to repair or uninstall installed software. Deleting them breaks uninstallers.",
        action_hint: "Leave in place unless you know exactly which patches are orphaned.",
        matcher: Matcher::AbsPath(&["%WINDIR%\\Installer"]),
        deletable: false,
    },
    Rule {
        id: "search-index",
        title: "Windows Search index",
        tier: Tier::Expert,
        rationale: "Full-text index of your files. Deleting forces a complete re-index (hours of background CPU/IO).",
        action_hint: "Rebuild via Settings → Search → Indexer troubleshooting instead.",
        matcher: Matcher::AbsPath(&[
            "%PROGRAMDATA%\\Microsoft\\Search\\Data",
        ]),
        deletable: false,
    },
];
