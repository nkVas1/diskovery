# Diskovery — Development Roadmap

Guiding principle: **every phase ends with a runnable app that is demonstrably better than before.**
Status: **v0.1.0 — all six phases landed in their core form.** Unchecked items form the → 1.0 backlog.

---

## Phase 0 — Foundation ✅

- [x] Research state of the art: WizTree MFT scanning, czkawka dedup pipeline, Tauri 2 maturity, Gemini pricing
- [x] Pivotal decisions: stack (Tauri 2 + Rust + React), name (**Diskovery**), public GitHub repo
- [x] Charter docs: README, ARCHITECTURE, AI-PRIVACY, this roadmap
- [x] Scaffold Tauri 2 app: Vite + React 19 + TypeScript (strict) + Tailwind CSS v4
- [x] CI (GitHub Actions): `cargo fmt --check`, `clippy -D warnings`, `tsc`, vite build
- [x] App shell: custom title bar, Abyss dark theme, nav rail

**Delivered:** `npm run tauri dev` opens the Diskovery shell.

---

## Phase 1 — Scan Engine ✅ (core)

- [x] Multi-threaded directory walker (rayon), reparse-point safe (no junction/symlink cycles)
- [x] Streaming IPC: progress events every 90 ms, live counters in UI
- [x] BFS arena tree with contiguous children (folder listing = slice, not traversal)
- [x] Scan dashboard: volume cards with capacity meters, live KPI tiles, top folders / files / extensions
- [ ] → 1.0: NTFS MFT fast path (elevated raw-volume read, WizTree-class)
- [ ] → 1.0: elevation flow (UAC prompt) for system-protected dirs

**Delivered:** full-drive scans with live telemetry; graceful skip-and-count on access-denied.

---

## Phase 2 — Treemap & Explorer ✅ (core)

- [x] Squarified layout (Bruls et al.) computed in the Rust core
- [x] Van Wijk cushion shading rasterized in-core; full RGBA frame shipped over IPC to canvas
- [x] Grid-indexed hit-testing, hover inspector, click-select, double-click drill-down
- [x] Breadcrumb navigation, depth-1 folder labels
- [x] Fixed-order 8-slot category palette (dataviz-validated, CVD-aware)
- [x] File ops: open, reveal in Explorer, delete to Recycle Bin (ancestor sizes update live)
- [ ] → 1.0: search & filters (size / age / type)
- [ ] → 1.0: WebGL2 renderer with animated zoom transitions

**Delivered:** the signature visualization with full inspect/act loop.

---

## Phase 3 — Duplicate Lab ✅ (core)

- [x] Three-stage pipeline: size groups → 16 KB BLAKE3 prehash → full BLAKE3 (parallel, mmap ≥ 4 MB)
- [x] Persistent hash cache (redb, keyed by path + size + mtime) → incremental re-runs
- [x] Hardlink awareness via NTFS file identity — zero false duplicates
- [x] Lab UI: staged progress, wasted-space totals, group cards, min-size filter
- [x] Keep-newest strategy + per-file recycle
- [ ] → 1.0: keep-oldest / by-folder-priority strategies, replace-with-hardlink (expert)

**Delivered:** byte-identical guarantees with cached re-scans.

---

## Phase 4 — Cleanup Advisor ✅ (core)

- [x] Embedded knowledge base: 21 curated rules (temp, browser/GPU/package caches, Windows.old,
      hiberfil, WinSxS, Docker/WSL, stale node_modules & cargo targets, big logs, disc images…)
- [x] Safety tiers 🟢 Safe / 🟡 Caution / 🔴 Expert — every finding carries a rationale + action hint
- [x] Matchers: env-expanded absolute paths, volume-root relative, dir-name with sibling +
      staleness conditions, extension + min-size
- [x] Advice-only findings for system-managed sinks (never offers to delete WinSxS & co.)
- [x] One-click recycle with per-item failure tolerance (files in use are skipped and counted)
- [ ] → 1.0: grow KB toward 40+ rules, restore-point advice, WinSxS true-size estimate

**Delivered:** real gigabytes found on a dev machine with zero harmful suggestions.

---

## Phase 5 — AI Insights ✅ (core)

- [x] Anonymized digest builder: categories, extensions, sanitized top folders, age profile,
      duplicates & advisor stats — user names replaced by `<dirN>` tokens, map stays on-device
- [x] Gemini client: `gemini-3.1-flash-lite`, structured JSON output schema, retry/backoff
- [x] Data passport: the exact payload + token estimate, shown before anything is sent
- [x] Per-scan report cache; strict opt-in (nothing sent until the button is clicked)
- [x] Settings vault for the API key (settings → env → .env resolution), RU/EN response language
- [ ] → 1.0: "Explain this folder" contextual action, token-level cost meter

**Delivered:** one click → prioritized expert plan; payload provably anonymous.

---

## Phase 6 — Polish & Release ✅ (0.1)

- [x] a11y: visible keyboard focus, `prefers-reduced-motion` support
- [x] NSIS installer via Tauri bundler
- [x] `v0.1.0` tagged GitHub release
- [ ] → 1.0: UI i18n (EN/RU) — AI answers already speak both
- [ ] → 1.0: screenshots/GIFs in README, perf profiling passes, auto-update

---

## Beyond 1.0 (idea backlog)

- Similar-media detection (perceptual hashes for images/video)
- Scan snapshots & diffing over time ("what grew this month?")
- Portable mode, CLI companion, scheduled background scans
- Local LLM fallback (zero-cloud mode)
