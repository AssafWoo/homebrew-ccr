# Changelog

## v1.3.0

### New Features

- **Read Delta Mode**: File re-reads now send unified diffs instead of full file content. Unchanged re-reads return structural digests (function/class signatures). Both activate automatically — no config needed.
- **Structural Map**: New `structure_map` module extracts function/struct/class/type signatures for Rust, Python, TypeScript/JS, Go, Java, Ruby, and C/C++ files. Used by delta mode on unchanged re-reads.
- **Pre-Compaction Digest** *(Claude Code only)*: PandaFilter captures session state (edited files, error signatures, top commands) before Claude auto-compacts and restores it in the new session via `additionalContext`. Installed automatically with `panda init`.
- **MoE Sparse Filter Router** *(opt-in)*: Content-aware router analyzes each input and activates only the most relevant filter strategies (error-focus, dedup, structural digest, semantic summary, tree compress, pass-through). Enable with `use_router = true` in `panda.toml`.
- **Expert Collapse Detection**: Tracks per-expert activation counts. When one expert exceeds 70% share, a noise bonus prevents collapse. Enable with `router_exploration_noise = true`.
- **Quality Score**: `panda gain` now shows a multi-signal quality grade (S/A/B/C/D/F) based on compression ratio, cache hit rate, and delta re-read rate. Full breakdown in `panda gain --insight`.

### New Agent

- **OpenClaw support**: `panda init --agent openclaw` installs pre/post tool-use hooks into `~/.openclaw/openclaw.json`. Detected automatically by `panda init --agent all`.

### Improvements

- `panda gain --insight` now includes a Quality Score section with per-signal bars and actionable tips.
- `panda gain` summary shows a one-line quality banner.
- `panda init` registers `PreCompact` and `SessionStart` lifecycle hooks for compaction digest (Claude Code).
- Session state now includes a file content cache (20 files, 20 KB/file) for delta mode.

### Internals

- New `ccr-core/src/delta.rs` — LCS-based unified diff with 1-line context, `TooLarge`/`Unchanged`/`NotEligible` variants.
- New `ccr-core/src/structure_map.rs` — structural signature extraction for 8 language families.
- New `ccr-core/src/router.rs` — `ContentFeatures`, `score_experts()`, `top_k_sparse()`, `exploration_bonus()`, `compute_hhi()`.
- `ccr-core/src/config.rs` — `use_router` and `router_exploration_noise` flags in `GlobalConfig`.
- `ccr/src/session.rs` — `FileCacheEntry`, `file_content_cache`, `extract_digest()`, `SessionDigest`.
- `ccr/src/analytics_db.rs` — `get_quality_signals()` for quality score computation.

---

## v1.2.2

See git log for prior changes.
