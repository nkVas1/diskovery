<div align="center">

# 💠 Diskovery

**See your disk. Understand it. Reclaim it.**

A modern disk-space intelligence app for Windows — blazing-fast scanning, GPU-accelerated treemap,
professional duplicate detection and an AI advisor that explains *what* eats your space and *what is safe to clean*.

[![Status](https://img.shields.io/badge/status-in%20development-blueviolet)](docs/ROADMAP.md)
[![Stack](https://img.shields.io/badge/Tauri%202-Rust%20%2B%20React-orange)](docs/ARCHITECTURE.md)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-0078d4)

*A spiritual successor to WinDirStat, rebuilt for 2026.*

</div>

---

## Why Diskovery?

|                  | WinDirStat (2003)  | WizTree        | **Diskovery**                    |
|------------------|--------------------|----------------|----------------------------------|
| Scan speed       | slow directory walk| ⚡ MFT          | ⚡ MFT + parallel walker fallback |
| Treemap          | CPU cushion        | basic          | GPU-accelerated, animated        |
| Duplicates       | —                  | basic          | BLAKE3 three-stage pipeline      |
| Cleanup guidance | —                  | —              | safety-tiered advisor            |
| AI insights      | —                  | —              | privacy-first Gemini analysis    |
| Open source      | ✓                  | ✗              | ✓ MIT                            |

## Feature pillars

1. **Turbo scan** — reads the NTFS Master File Table directly (seconds for a 1 TB drive), with a
   multi-threaded directory walker as fallback for non-NTFS volumes and non-elevated sessions.
2. **Living treemap** — squarified, cushion-shaded treemap rendered on GPU: smooth animated
   drill-down, hover inspector, category color system.
3. **Duplicate lab** — three-stage detection (size groups → 2 KB BLAKE3 prehash → full BLAKE3),
   persistent hash cache between sessions, smart keep-strategies, hardlink-aware.
4. **Cleanup advisor** — a curated knowledge base of Windows space sinks, each rated
   **🟢 Safe / 🟡 Caution / 🔴 Expert**: temp files, browser & shader caches, `Windows.old`,
   hibernation file, package-manager caches, stale `node_modules`, Docker/WSL disks and more.
5. **AI insights (opt-in)** — Gemini analyzes an *anonymized statistical digest* of your scan and
   returns a prioritized, human-readable cleanup plan. No file contents, no personal paths,
   tiny token budget. See [AI & Privacy](docs/AI-PRIVACY.md).

## Status

✅ **v0.1.0 — feature-complete core.** Turbo scan, cushion treemap, duplicate lab,
safety-tiered advisor and AI insights are all working. The road to 1.0 (MFT fast
path, UI i18n, WebGL treemap, auto-update) lives in the [Roadmap](docs/ROADMAP.md).

## Architecture at a glance

```text
┌───────────── Frontend — React 19 + TypeScript (WebView2) ─────────────┐
│  GPU treemap · scan dashboard · duplicate lab · advisor · AI panel    │
├────────────────────────── typed Tauri IPC ────────────────────────────┤
│  Rust core                                                            │
│  scanner (MFT + parallel walk) · dedup (BLAKE3 pipeline + cache)      │
│  advisor (rules KB) · ai (digest + privacy filter + Gemini) · store   │
└───────────────────────────────────────────────────────────────────────┘
```

Details: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)

## Quick start (dev)

> Prerequisites: Rust (stable), Node.js ≥ 20.

```bash
git clone https://github.com/nkVas1/diskovery.git
cd diskovery
npm install
npm run tauri dev
```

Release build with NSIS installer: `npm run tauri build`.
For AI Insights, paste a [Gemini API key](https://aistudio.google.com/apikey) in
Settings (or set `GOOGLE_GENERATIVE_AI_API_KEY` in a local `.env`).

## Documentation

- [Roadmap](docs/ROADMAP.md) — phases, milestones, definitions of done
- [Architecture](docs/ARCHITECTURE.md) — modules, data flow, key decisions
- [AI & Privacy](docs/AI-PRIVACY.md) — exactly what the AI sees and never sees

## License

[MIT](LICENSE) © 2026 [nkVas1](https://github.com/nkVas1)
