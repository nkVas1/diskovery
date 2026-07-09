# AI & Privacy

Diskovery's AI layer is **opt-in** and built on one rule:
**statistics leave the machine — identities never do.**

## What the AI receives

A compact JSON digest (~2–4K tokens) per scan:

- **Volume totals** — capacity, used, free, drive type (SSD/HDD)
- **Extension histogram** — top-N by bytes and count (`".mp4": 212 GB / 1,844 files`)
- **Category rollups** — video, images, code, archives, documents, games, system
- **Top folders by size** — path-sanitized (see below), depth-limited
- **Age profile** — bytes untouched for > 1 / 2 / 5 years
- **Duplicates summary** — group count, wasted bytes, dominant categories
- **Advisor findings** — rule IDs and reclaimable-byte estimates
- **OS context** — Windows version only

## What the AI never receives

- File contents — never read for AI purposes
- File names
- User-created folder names (replaced by typed tokens)
- Usernames, hostnames, serial numbers, emails, IPs

## Sanitization rules

- Well-known system/app folders keep their real names: `Windows`, `Program Files`,
  `node_modules`, `.gradle`, `AppData\Local\Temp`, …
- User-created names are replaced with typed tokens preserving only what analysis needs:
  `<user>/Documents/<folder#1 · 42 GB · mostly video>`
- The mapping token → real path stays local, so AI recommendations can be resolved back
  to real folders **on the device only**.

## Data passport

Before the first request (and any time on demand) the UI shows the **exact payload** that will be
sent. Nothing leaves the machine without explicit approval; a global toggle disables the AI layer
entirely. When disabled, Diskovery remains fully functional — the local advisor and all heuristics
work offline.

## Token & cost discipline

- Model: **`gemini-3.1-flash-lite`** — cheapest tier, 1M context, free-tier quota available
- One digest per scan; cached and reused for follow-up questions
- Structured JSON responses with bounded output size
- Retry with backoff; hard budget cap per session
