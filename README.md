# CCR — Cool Cost Reduction

> **60–95% token savings on Claude Code and Cursor tool outputs.** CCR sits between the agent and your tools, compressing what the model reads without changing what you ask it to do.

---

## Token Savings

Numbers from `ccr/tests/handler_benchmarks.rs` — each handler fed a realistic large-project fixture. Run `cargo test -p ccr benchmark -- --nocapture` to reproduce, or `ccr gain` to see your own live data.

| Operation | Without CCR | With CCR | Savings |
|-----------|------------:|---------:|:-------:|
| `pip install` | 1,787 | 9 | **−99%** |
| `uv sync` | 1,574 | 15 | **−99%** |
| `playwright test` | 1,367 | 19 | **−99%** |
| `gradle build` | 803 | 17 | **−98%** |
| `go test` | 4,507 | 148 | **−97%** |
| `pytest` | 3,818 | 162 | **−96%** |
| `terraform plan` | 3,926 | 163 | **−96%** |
| `npm install` | 648 | 25 | **−96%** |
| `cargo build` | 1,923 | 93 | **−95%** |
| `cargo test` | 2,782 | 174 | **−94%** |
| `next build` | 549 | 53 | **−90%** |
| `cargo clippy` | 786 | 93 | **−88%** |
| `make` | 545 | 72 | **−87%** |
| `git push` | 173 | 24 | **−86%** |
| `ls` | 691 | 102 | **−85%** |
| `webpack` | 882 | 143 | **−84%** |
| `vitest` | 625 | 103 | **−84%** |
| `nx run-many` | 1,541 | 273 | **−82%** |
| `turbo run build` | 597 | 115 | **−81%** |
| `ruff check` | 2,035 | 435 | −79% |
| `eslint` | 4,393 | 974 | −78% |
| `git log` | 1,573 | 353 | −78% |
| `grep` | 2,925 | 691 | −76% |
| `helm install` | 224 | 54 | −76% |
| `docker ps` | 1,057 | 266 | −75% |
| `golangci-lint` | 3,678 | 960 | −74% |
| `git status` | 650 | 184 | −72% |
| `kubectl get pods` | 2,306 | 689 | −70% |
| `vite build` | 526 | 182 | −65% |
| `jest` | 330 | 114 | −65% |
| `env` | 1,155 | 399 | −65% |
| `mvn install` | 4,585 | 1,613 | −65% |
| `brew install` | 368 | 148 | −60% |
| `gh pr list` | 774 | 321 | −59% |
| `git diff` | 6,370 | 2,654 | −58% |
| `biome lint` | 1,503 | 753 | −50% |
| `tsc` | 2,598 | 1,320 | −49% |
| `mypy` | 2,053 | 1,088 | −47% |
| `stylelint` | 1,100 | 845 | −23% |
| **Total** | **69,727** | **15,846** | **−77%** |

---

## How It Works

```
Claude runs: cargo build
    ↓ PreToolUse hook rewrites to: ccr run cargo build
    ↓ ccr run executes cargo, filters output through Cargo handler
    ↓ Claude reads: errors + warning count only (~87% fewer tokens)

Claude runs: Read file.rs  (large file)
    ↓ PostToolUse hook: BERT pipeline using current task as query
    ↓ Claude reads: compressed file content focused on what's relevant

Claude runs: git status  (seen recently)
    ↓ PreToolUse hook rewrites to: ccr run git status
    ↓ Pre-run cache hit (same HEAD+staged+unstaged hash)
    ↓ Claude reads: [PC: cached from 2m ago — ~1.8k tokens saved]
```

After `ccr init`, **this is fully automatic** — no changes to how you use Claude Code or Cursor.

CCR is local-only. It never sends data anywhere. The hook reads only tool stdout/stderr and (for BERT relevance queries) the agent's single most-recent message.

---

## Installation

### Homebrew (macOS — recommended)

```bash
brew tap AssafWoo/ccr
brew install ccr
```

That's it. `post_install` automatically runs `ccr init` (Claude Code) and `ccr init --agent cursor` (Cursor, if installed). Both agents are set up in one step.

If you need to re-run manually:

```bash
ccr init                      # Claude Code
ccr init --agent cursor       # Cursor
```

### Script (Linux / any platform)

```bash
curl -fsSL https://raw.githubusercontent.com/AssafWoo/homebrew-ccr/main/install.sh | bash
```

Installs Rust via `rustup` if needed, builds from source, adds `~/.cargo/bin` to PATH, and runs `ccr init`.

> **First run:** CCR downloads the BERT model (~90 MB, `all-MiniLM-L6-v2`) from HuggingFace and caches it at `~/.cache/huggingface/`. Subsequent runs are instant.

---

## FAQ

**Does CCR degrade Claude's output quality?**
No. CCR only removes noise — build logs, module graphs, passing test lines, progress bars. Errors, file paths, and summaries are always kept.

**What happens with a tool CCR doesn't know about?**
BERT semantic routing compares the command name against all known handlers. If confidence is high enough the closest handler is applied; otherwise the output passes through unchanged. CCR never silently drops output.

**How do I verify it's working?**
`ccr gain` after a session. To inspect what the model actually receives from a specific command: `ccr proxy git log --oneline -20`.

**Does CCR send any data outside my machine?**
Never. All processing is fully local. BERT runs on-device.

---

## Commands

### ccr gain

```bash
ccr gain                    # overall summary
ccr gain --breakdown        # include per-command table
ccr gain --history          # last 14 days
ccr gain --history --days 7
```

```
CCR Token Savings
═════════════════════════════════════════════════
  Runs:           315  (avg 280ms)
  Tokens saved:   32.9k / 71.1k  (46.3%)  ███████████░░░░░░░░░░░░░
  Cost saved:     ~$0.099  (at $3.00/1M)
  Today:          142 runs · 6.8k saved · 23.9%
  Top command:    (pipeline)  65.2%  ·  25.8k saved
```

Pricing uses `cost_per_million_tokens` from `ccr.toml` if set, otherwise `ANTHROPIC_MODEL` env var (Opus 4.6: $15, Sonnet 4.6: $3, Haiku 4.5: $0.80), otherwise $3.00.

### ccr init

```bash
ccr init                        # install into Claude Code (~/.claude/settings.json)
ccr init --agent cursor         # install into Cursor (~/.cursor/hooks.json)
ccr init --uninstall            # remove Claude Code hooks
ccr init --agent cursor --uninstall  # remove Cursor hooks
```

Safe to re-run — replaces existing CCR entries without touching other tools' hooks. Also writes an SHA-256 integrity baseline (see `ccr verify`).

### ccr verify

```bash
ccr verify
```

Checks hook integrity for both agents:

```
Claude Code:
  OK  Verified   /Users/you/.claude/hooks/ccr-rewrite.sh

Cursor:
  OK  Verified   /Users/you/.cursor/hooks/ccr-rewrite.sh
```

Exits 1 if either script has been tampered with. At hook invocation time, CCR silently verifies and exits 1 if tampering is detected.

### ccr compress

```bash
ccr compress --scan-session --dry-run   # estimate savings for current conversation
ccr compress --scan-session             # compress and write to {file}.compressed.json
ccr compress conversation.json -o out.json
```

Finds the most recently modified conversation JSONL under `~/.claude/projects/`, runs tiered compression (recent turns preserved verbatim, older turns compressed). When context pressure is high, the hook suggests running this automatically.

### ccr discover

```bash
ccr discover
```

Scans `~/.claude/projects/*/` JSONL history for Bash commands that ran without CCR. Reports estimated missed savings sorted by impact.

### ccr noise

```bash
ccr noise           # show learned noise patterns for this project
ccr noise --reset   # clear all patterns
```

Lines seen ≥10 times with ≥90% suppression rate are promoted to permanent pre-filters. Error/warning/panic lines are never promoted.

### ccr expand

```bash
ccr expand ZI_3       # print original lines from a collapsed block
ccr expand --list     # list all available IDs in this session
```

When CCR collapses output it embeds an ID: `[5 lines collapsed — ccr expand ZI_3]`

### ccr read-file

```bash
ccr read-file src/main.rs --level auto
ccr read-file src/large_module.rs --level aggressive
cat file.py | ccr read-file - --level strip
```

Applies the read-level filter and prints token savings to stderr. Levels: `passthrough`, `auto`, `strip`, `aggressive`.

### ccr filter / ccr run / ccr proxy

```bash
cargo clippy 2>&1 | ccr filter --command cargo
ccr run git status    # run through CCR handler
ccr proxy git status  # run raw (no filtering), record analytics baseline
```

---

## Handlers

48 handlers (60+ command aliases) in `ccr/src/handlers/`. Lookup cascade:

1. **User filters** — `.ccr/filters.toml` or `~/.config/ccr/filters.toml`
2. **Exact match** — direct command name
3. **Static alias table** — versioned binaries, wrappers, common aliases
4. **BERT routing** — unknown commands matched by embedding similarity

**TypeScript / JavaScript**

| Handler | Keys | Key behavior |
|---------|------|-------------|
| **tsc** | `tsc` | Groups errors by file; deduplicates repeated TS codes. `Build OK` on clean. |
| **vitest** | `vitest` | FAIL blocks + summary; drops `✓` lines. |
| **jest** | `jest`, `bun`, `deno` | `●` failure blocks + summary; drops `PASS` lines. |
| **nx** | `nx`, `npx nx` | Passing tasks collapsed to `[N tasks passed]`; failing task output kept. |
| **eslint** | `eslint` | Errors grouped by file, caps at 20 + `[+N more]`. |
| **next** | `next` | `build`: route table collapsed, errors + page count. `dev`: errors + ready line. |
| **playwright** | `playwright` | Failing test names + error messages; passing tests dropped. |
| **prettier** | `prettier` | `--check`: files needing formatting + count. |
| **vite** | `vite` | Asset chunk table collapsed, HMR deduplication. |
| **webpack** | `webpack` | Module resolution graph dropped; keeps assets, errors, build result. |
| **turbo** | `turbo` | Inner task output stripped; cache hit/miss per package + final summary. |
| **stylelint** | `stylelint` | Issues grouped by file, caps at 40 + `[+N more]`. |
| **biome** | `biome` | Code context snippets stripped; keeps file:line, rule, message. |

**Ruby**

| Handler | Keys | Key behavior |
|---------|------|-------------|
| **rspec** | `rspec` | Injects `--format json`; example-level failures with message + location. |
| **rubocop** | `rubocop` | Injects `--format json`; offenses grouped by severity, capped. |
| **rake** | `rake`, `bundle` | Failure/error blocks + summary; drops passing test lines. |

**Python**

| Handler | Keys | Key behavior |
|---------|------|-------------|
| **pytest** | `pytest` | FAILED node IDs + AssertionError + short summary. |
| **uv** | `uv`, `uvx` | Strips Downloading/Fetching/Preparing noise; keeps errors + final summary. |
| **ruff** | `ruff` | Violations grouped by error code. `format`: summary line only. |
| **mypy** | `mypy` | Errors grouped by file, capped at 10 per file. |
| **pip** | `pip`, `poetry`, `pdm`, `conda` | `install`: `[complete — N packages]` or already-satisfied short-circuit. |
| **python** | `python` | Traceback: keep block + final error. Long output: BERT. |

**DevOps / Cloud**

| Handler | Keys | Key behavior |
|---------|------|-------------|
| **kubectl** | `kubectl`, `k` | Smart column selection, log anomaly scoring, describe key sections. |
| **gh** | `gh` | Compact tables for list commands; strips HTML noise from `pr view`. |
| **terraform** | `terraform`, `tofu` | `plan`: `+`/`-`/`~` + summary. `validate`: short-circuits on success. |
| **aws** | `aws`, `gcloud`, `az` | Action-specific resource extraction; `--output json` injected for read-only actions. |
| **make** | `make`, `ninja` | "Nothing to be done" short-circuit; keeps errors + recipe failures. |
| **go** | `go` | `test`: injects `-json`, FAIL blocks + summary. `build`: errors only. |
| **golangci-lint** | `golangci-lint` | Diagnostics grouped by file; runner noise dropped. |
| **prisma** | `prisma` | `generate`/`migrate`/`db push` structured summaries. |
| **mvn** | `mvn` | Drops `[INFO]` noise; keeps errors + reactor summary. |
| **gradle** | `gradle` | UP-TO-DATE tasks collapsed; FAILED tasks and errors kept. |
| **helm** | `helm` | `list`: compact table. `status`/`diff`/`template`: structured. |

**System / Utility**

| Handler | Keys | Key behavior |
|---------|------|-------------|
| **cargo** | `cargo` | `build`/`clippy`: JSON format, errors + warning count. `test`: failures + summary. |
| **git** | `git` | `status`: counts. `log`: `--oneline`, cap 20. `diff`: 2 context lines, 200-line cap. |
| **curl** | `curl` | JSON → type schema. Non-JSON: cap 30 lines. |
| **docker** | `docker` | `logs`: ANSI strip + BERT. `ps`/`images`: formatted tables. |
| **npm/yarn** | `npm`, `yarn` | `install`: package count; strips boilerplate. |
| **pnpm** | `pnpm` | `install`: summary; drops progress bars. |
| **journalctl** | `journalctl` | Injects `--no-pager -n 200`. BERT anomaly scoring. |
| **psql** | `psql` | Strips borders, caps at 20 rows. |
| **brew** | `brew` | `install`/`update`: status lines + Caveats. |
| **tree** | `tree` | Auto-injects `-I "node_modules\|.git\|target\|..."`. |
| **diff** | `diff` | `+`/`-`/`@@` + 2 context lines, max 5 hunks. |
| **jq** | `jq` | Array: schema of first element + `[N items]`. |
| **env** | `env` | Categorized sections; sensitive values redacted. |
| **ls** | `ls` | Drops noise dirs; top-3 extension summary. |
| **grep / rg** | `grep`, `rg` | Compact paths, per-file 25-match cap. |
| **find** | `find` | Groups by directory, caps at 50. |
| **log** | `log` | Timestamp/UUID normalization, dedup `[×N]`, error summary block. |

---

## Pipeline Architecture

```
0. Hard input ceiling (200k chars — truncates before any stage)
1. Strip ANSI codes
2. Normalize whitespace
2.5 Global regex pre-filter (progress bars, spinners, download lines, decorators)
3. Command-specific pattern filter
4. If over summarize_threshold_lines:
   4a. BERT noise pre-filter
   4b. Entropy-adaptive BERT summarization
5. Hard output cap (50k chars)
```

**Minimum token gate:** Outputs under 15 tokens skip the entire pipeline. Step 4b runs up to 7 BERT passes (noise, clustering, entropy, anomaly, anchors, centroid, delta). Falls back to head+tail if BERT is unavailable.

---

## Configuration

Config is loaded from: `./ccr.toml` → `~/.config/ccr/config.toml` → embedded default.

```toml
[global]
summarize_threshold_lines = 50
head_lines = 30
tail_lines = 30
strip_ansi = true
normalize_whitespace = true
deduplicate_lines = true
input_char_ceiling = 200000
output_char_cap = 50000
# cost_per_million_tokens = 15.0

[tee]
enabled = true
mode = "aggressive"   # "aggressive" | "always" | "never"
max_files = 20

[read]
# "passthrough" (default) | "auto" | "strip" | "aggressive"
mode = "auto"

[commands.git]
patterns = [
  { regex = "^(Counting|Compressing|Receiving|Resolving) objects:.*", action = "Remove" },
]

[commands.cargo]
patterns = [
  { regex = "^\\s+Compiling \\S+ v[\\d.]+", action = "Collapse" },
  { regex = "^\\s+Downloaded \\S+ v[\\d.]+", action = "Remove"   },
]
```

Pattern actions: `Remove`, `Collapse`, `ReplaceWith = "text"`.

---

## User-Defined Filters

Place a `filters.toml` at `.ccr/filters.toml` (project-local) or `~/.config/ccr/filters.toml` (global). Project-local overrides global for the same key. Runs before any built-in handler.

```toml
[commands.myapp]
strip_lines_matching = ["DEBUG:", "TRACE:"]
keep_lines_matching  = []
max_lines = 50
on_empty  = "(no relevant output)"

[commands.myapp.match_output]
pattern        = "Server started"
message        = "ok — server ready"
unless_pattern = "error"
```

---

## Session Intelligence

State is tracked across turns via `CCR_SESSION_ID=$PPID`, stored at `~/.local/share/ccr/sessions/<id>.json`.

- **Result cache** — Post-pipeline bytes are frozen per input hash, returned identically on repeat calls to prevent prompt cache busts.
- **Semantic delta** — Repeated commands emit only new/changed lines: `[Δ from turn N: +M new, K repeated — ~T tokens saved]`.
- **Cross-turn dedup** — Identical outputs (cosine > 0.92) collapse to `[same output as turn 4 (3m ago) — 1.2k tokens saved]`.
- **Elastic context** — As session tokens grow, pipeline pressure scales 0 → 1, shrinking BERT budgets. At >80% pressure: `[⚠ context near full — run ccr compress --scan-session]`.
- **Intent-aware query** — Reads the agent's last message from the live session JSONL and uses it as the BERT query.

---

## Hook Architecture

### Claude Code

- Config: `~/.claude/settings.json`
- Script: `~/.claude/hooks/ccr-rewrite.sh`
- PreToolUse output format: `{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "allow", "updatedInput": {...}}}`

### Cursor

- Config: `~/.cursor/hooks.json`
- Script: `~/.cursor/hooks/ccr-rewrite.sh`
- PreToolUse output format: `{"permission": "allow", "updated_input": {...}}`
- Cursor requires valid JSON on all code paths — the script returns `{}` when no rewrite applies (Claude Code uses `exit 0`).
- PostToolUse entries use `CCR_AGENT=cursor` so the hook checks `~/.cursor` integrity instead of `~/.claude`.

Both agents share the same binary and compression pipeline. PostToolUse output format (`{"output": "..."}`) is identical for both.

### PreToolUse (both agents)

- **Known handler** → rewrites to `ccr run <cmd>`
- **Unknown** → no-op (original command used)
- **Compound commands** → each segment rewritten independently
- **Already wrapped** → no double-wrap

### PostToolUse (both agents)

- **Bash** — min-token gate → result cache → noise pre-filter → BERT pipeline → delta compression → session cache → analytics
- **Read** — files < 50 lines pass through; read-level early-exit if configured; otherwise BERT + session dedup
- **Glob** — results ≤ 20 pass through; larger lists grouped by directory, session dedup
- **Grep** — results ≤ 10 lines pass through; larger results through GrepHandler

### Hook Integrity

`ccr init` writes SHA-256 baselines to `~/.claude/hooks/.ccr-hook.sha256` and `~/.cursor/hooks/.ccr-hook.sha256` (chmod 0o444). At every hook invocation, CCR verifies the script and exits 1 with a warning if tampered. `ccr verify` checks both agents.

---

## Crate Overview

```
ccr/            CLI binary — handlers, hooks, session state, commands
ccr-core/       Core library (no I/O) — pipeline, BERT summarizer, config, analytics
ccr-sdk/        Conversation compression — tiered compressor, deduplicator, Ollama
ccr-eval/       Evaluation suite — fixtures against Claude API
config/         Embedded default filter patterns
```

---

## Uninstall

```bash
ccr init --uninstall                      # remove Claude Code hooks
ccr init --agent cursor --uninstall       # remove Cursor hooks

brew uninstall ccr && brew untap AssafWoo/ccr   # Homebrew
# or
cargo uninstall ccr                             # cargo install

# optional — remove cached data
rm -rf ~/.local/share/ccr
rm -rf ~/.cache/huggingface/hub/models--sentence-transformers--all-MiniLM-L6-v2
```

---

## Contributing

Open an issue or PR on [GitHub](https://github.com/AssafWoo/homebrew-ccr). To add a handler: implement the `Handler` trait and register it in `ccr/src/handlers/mod.rs` — see `git.rs` as a template.

---

## License

MIT — see [LICENSE](LICENSE).
