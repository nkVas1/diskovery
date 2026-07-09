# Changelog

All notable changes to Diskovery are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning: [SemVer](https://semver.org).

## [0.1.0] — 2026-07-09

First public release: the feature-complete core.

### Added

- **Scan engine** — rayon-parallel directory walker (reparse-safe), BFS arena tree,
  live progress streaming (files/s, bytes, current path) at 90 ms cadence,
  volume list with capacity meters.
- **Cushion treemap** — squarified layout + Van Wijk cushion shading rasterized in
  the Rust core and shipped to canvas as a full RGBA frame; grid-indexed hit-testing,
  breadcrumb drill-down, depth-1 labels, CVD-aware 8-slot category palette;
  open / reveal / recycle right from the map.
- **Duplicate lab** — size groups → 16 KB BLAKE3 prehash → parallel full BLAKE3
  (mmap ≥ 4 MB), persistent redb hash cache, NTFS hardlink awareness, keep-newest
  strategy, per-file recycle.
- **Cleanup advisor** — 21-rule embedded knowledge base with Safe / Caution / Expert
  tiers, rationale + action hint on every finding, advice-only mode for
  system-managed sinks, one-click recycle with in-use-file tolerance.
- **AI insights** — anonymized statistical digest (token substitution for user
  folder names; mapping never leaves the device), Gemini `gemini-3.1-flash-lite`
  with structured JSON output, data-passport preview, per-scan report cache,
  RU/EN response language, API-key vault in Settings.
- **App shell** — Tauri 2 + React 19 + Tailwind v4, custom title bar, Abyss
  dark theme, keyboard-visible focus, reduced-motion support, NSIS installer.

### Security & privacy

- All deletions go to the Recycle Bin; no permanent deletes anywhere.
- The AI layer is strictly opt-in; file names and contents never leave the machine.
