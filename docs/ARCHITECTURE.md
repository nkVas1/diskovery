# Diskovery — Architecture

Tauri 2 desktop application: a **Rust core** does all heavy lifting (scanning, hashing, rules, AI
digest), a **React frontend** does all presentation, connected by typed IPC. The web layer never
touches the filesystem directly.

```
┌───────────── Frontend — React 19 + TypeScript (WebView2) ─────────────┐
│  GPU treemap · scan dashboard · duplicate lab · advisor · AI panel    │
├────────────────────────── typed Tauri IPC ────────────────────────────┤
│  Rust core                                                            │
│  scanner (MFT + parallel walk) · dedup (BLAKE3 pipeline + cache)      │
│  advisor (rules KB) · ai (digest + privacy filter + Gemini) · store   │
└───────────────────────────────────────────────────────────────────────┘
```

## Repository layout (target)

```
diskovery/
├─ src/                       # React frontend
│  ├─ app/                    # shell, routing, theming
│  ├─ features/               # scan/ · treemap/ · duplicates/ · advisor/ · ai/
│  └─ shared/                 # UI kit, hooks, generated IPC bindings
├─ src-tauri/
│  └─ src/
│     ├─ scanner/             # parallel walker + NTFS MFT reader
│     ├─ dedup/               # 3-stage pipeline + persistent cache
│     ├─ advisor/             # rules engine + embedded knowledge base
│     ├─ ai/                  # digest builder, privacy filter, Gemini client
│     ├─ store/               # settings, scan/hash cache (redb)
│     └─ ipc/                 # commands, events, DTOs
├─ docs/                      # this documentation
└─ .github/workflows/         # CI
```

## Modules

### scanner
- **Fast path:** raw NTFS volume handle → Master File Table parse (requires elevation).
  Produces the full file table in seconds (WizTree-class); sizes aggregate bottom-up.
- **Fallback:** multi-threaded directory walk (rayon), long-path (`\\?\`) aware, with
  junction/symlink cycle protection — works on any filesystem without admin rights.
- Emits progress events (files/s, bytes, current path) over a Tauri channel; the UI renders
  incrementally instead of waiting for completion.

### dedup
Three-stage pipeline (czkawka-inspired — the proven cost minimizer):

1. **Size grouping** — only same-size files can be duplicates; 0-byte files and hardlinks to the
   same NTFS file ID are excluded up front.
2. **Prehash** — BLAKE3 of the first 2 KB prunes roughly half of the candidates cheaply.
3. **Full hash** — parallel, memory-mapped full BLAKE3; equal hashes ⇒ byte-identical groups.

A persistent cache in **redb**, keyed by `(volume id, file id, size, mtime)`, makes repeat scans
incremental: unchanged files are never re-hashed.

### advisor
Data-driven knowledge base embedded at compile time (one rule = matcher + safety tier +
rationale + suggested action). The engine evaluates rules against the scan tree, estimates
reclaimable bytes and merges overlapping findings. Tiers:

- 🟢 **Safe** — removable with no functional impact (temp, caches that rebuild themselves)
- 🟡 **Caution** — removable with understood consequences (hibernation file, old restore data)
- 🔴 **Expert** — for users who know exactly what they're doing (WinSxS, app data of live apps)

### ai
`digest builder → privacy filter → Gemini client`. The AI module's public API accepts only
aggregated digest structs — raw paths and file names are not representable in its input types,
making the privacy boundary structural rather than conventional. Model:
**`gemini-3.1-flash-lite`** (structured JSON output; the `-preview` variant was retired
2026-07-09). API key comes from local settings/env — never committed. Full data policy:
[AI-PRIVACY.md](AI-PRIVACY.md).

### Frontend
React 19 + strict TypeScript, Vite, Tailwind CSS v4, Zustand for state, Motion for transitions.
The treemap layout is computed in Rust and rendered on a WebGL2 canvas (cushion shading as an
homage to WinDirStat); DOM is used only for overlays and controls.

## Key decisions (mini-ADRs)

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | **Tauri 2** over Electron / WinUI 3 | ~10 MB vs 150 MB, ×5 less RAM; the Rust core *is* the product advantage (scan/hash performance); full creative freedom for a signature UI |
| 2 | **BLAKE3** for hashing | Fastest cryptographic hash (SIMD, parallel); collision-safe for dedup, unlike CRC32 |
| 3 | **MFT fast path + walker fallback** | WizTree-class speed where possible, universal correctness everywhere else |
| 4 | **redb** for caches | Pure-Rust embedded KV store, zero native deps; revisit if relational queries emerge |
| 5 | **Privacy filter as a type boundary** | The AI client physically cannot receive raw paths — enforced by the type system, not by discipline |
| 6 | **Recycle Bin first** | Every destructive action is reversible by default; permanent deletion is an explicit expert action |
