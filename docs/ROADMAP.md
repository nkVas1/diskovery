# Diskovery — Development Roadmap

Guiding principle: **every phase ends with a runnable app that is demonstrably better than before.**
Order is fixed; timing is flexible. Checkboxes are updated as work lands.

---

## Phase 0 — Foundation ✦ *current*

Goal: a project skeleton that compiles, launches and is properly hosted.

- [x] Research state of the art: WizTree MFT scanning, czkawka dedup pipeline, Tauri 2 maturity, Gemini pricing
- [x] Pivotal decisions: stack (Tauri 2 + Rust + React), name (**Diskovery**), public GitHub repo
- [x] Charter docs: README, ARCHITECTURE, AI-PRIVACY, this roadmap
- [ ] Scaffold Tauri 2 app: Vite + React 19 + TypeScript (strict) + Tailwind CSS v4
- [ ] CI (GitHub Actions): `cargo fmt --check`, `clippy`, `tsc --noEmit`, build — on PR & main
- [ ] App shell: window chrome, dark-first theme system, navigation skeleton

**Deliverable:** `npm run tauri dev` opens the Diskovery shell.

---

## Phase 1 — Scan Engine

Goal: scan any drive or folder fast and stream results live.

- [ ] Multi-threaded directory walker (rayon), long-path (`\\?\`) aware, junction/symlink cycle-safe
- [ ] NTFS MFT fast path (elevated): full-volume file table in seconds
- [ ] Streaming IPC: progress events (files/s, bytes, current path), incremental tree building in UI
- [ ] In-memory file tree with per-node aggregates (size, count, extension stats)
- [ ] Scan dashboard: live counters, top folders / files / extensions
- [ ] Elevation flow (UAC prompt) + graceful non-admin fallback

**Deliverable:** full `C:` scan with a live dashboard; MFT path ≥ 10× faster than the walker.

---

## Phase 2 — Treemap & Explorer

Goal: the signature visualization plus everyday file operations.

- [ ] Squarified treemap layout engine (computed in Rust, serialized to the UI)
- [ ] GPU renderer (WebGL2): cushion shading, 60 fps pan/zoom on 1M+ nodes
- [ ] Animated drill-down/up, hover inspector, breadcrumb path bar
- [ ] Category color system (media, code, archives, documents, system…)
- [ ] File operations: open, reveal in Explorer, properties, delete to Recycle Bin
- [ ] Search and filters (size / age / type)

**Deliverable:** fluid treemap navigation with full feature parity with classic WinDirStat.

---

## Phase 3 — Duplicate Lab

Goal: professional-grade duplicate detection.

- [ ] Three-stage pipeline: size groups → BLAKE3 prehash (first 2 KB) → full BLAKE3 (parallel, mmap)
- [ ] Persistent hash cache (redb, keyed by volume + file id + size + mtime) → incremental re-scans
- [ ] Hardlink/junction awareness (NTFS file IDs) — zero false duplicates
- [ ] Duplicate lab UI: groups, wasted-space totals, image/media previews
- [ ] Keep-strategies: newest / oldest / by folder priority / manual selection
- [ ] Actions: delete to Recycle Bin; replace with hardlink (expert mode)

**Deliverable:** multi-TB dedup pass in minutes with byte-identical guarantees.

---

## Phase 4 — Cleanup Advisor

Goal: tell the user what is safe to remove — with receipts.

- [ ] Data-driven knowledge base (embedded): 40+ known Windows space sinks
- [ ] Safety tiers: 🟢 Safe / 🟡 Caution / 🔴 Expert — each with a rationale and consequences
- [ ] Detectors: temp dirs, browser & shader caches, `Windows.old`, `hiberfil.sys`, Delivery
      Optimization, WinSxS estimate, Recycle Bin, npm/pip/cargo/gradle caches, stale
      `node_modules` / `target` dirs, installer leftovers, oversized logs
- [ ] Reclaimable-size estimation per finding + one-click cleanup for the Safe tier
- [ ] Reversibility first: Recycle Bin by default, restore-point advice for system items

**Deliverable:** the advisor finds real 5–50 GB on a typical dev machine with zero harmful suggestions.

---

## Phase 5 — AI Insights

Goal: Gemini turns statistics into an expert, personalized plan.

- [ ] Digest builder: anonymized scan summary (see [AI-PRIVACY](AI-PRIVACY.md)), ≤ ~4K tokens
- [ ] Gemini client: `gemini-3.1-flash-lite`, structured JSON output, retry/backoff
- [ ] Data passport UI: show the exact payload before sending; strict opt-in
- [ ] Insights panel: narrative analysis, prioritized action plan, risk notes
- [ ] “Explain this folder” contextual action
- [ ] Token discipline: digest cached per scan; no re-asking for unchanged data

**Deliverable:** one click → a readable expert report; the payload is provably anonymous.

---

## Phase 6 — Polish & Release 1.0

- [ ] Performance profiling passes (scan, treemap, memory footprint)
- [ ] i18n: EN + RU; accessibility: keyboard navigation, contrast, reduced motion
- [ ] Onboarding, empty states, error surfaces
- [ ] Installer (MSI/NSIS via Tauri bundler), auto-update strategy
- [ ] Screenshots/GIFs for README, `v1.0.0` GitHub release

**Deliverable:** public 1.0 with a showcase README.

---

## Beyond 1.0 (idea backlog)

- Similar-media detection (perceptual hashes for images/video)
- Scan snapshots & diffing over time (“what grew this month?”)
- Portable mode, CLI companion, scheduled background scans
- Local LLM fallback (zero-cloud mode)
