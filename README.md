# CCR — Cool Cost Reduction

> **60–95% token savings on Claude Code tool outputs.** CCR intercepts shell commands before Claude reads their output, routes them through specialized handlers, and returns compact summaries — without losing the information Claude actually needs.

---

## Contents

- [Why CCR](#why-ccr)
- [How It Works](#how-it-works)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Commands](#commands)
  - [ccr run](#ccr-run)
  - [ccr gain](#ccr-gain)
  - [ccr discover](#ccr-discover)
  - [ccr init](#ccr-init)
  - [ccr filter](#ccr-filter)
  - [ccr proxy](#ccr-proxy)
- [Handlers](#handlers)
  - [cargo](#cargo)
  - [git](#git)
  - [curl](#curl)
  - [docker](#docker)
  - [npm / pnpm / yarn](#npm--pnpm--yarn)
  - [ls](#ls)
  - [cat](#cat)
  - [grep / rg](#grep--rg)
  - [find](#find)
- [Pipeline (Unknown Commands)](#pipeline-unknown-commands)
- [Configuration](#configuration)
- [Analytics](#analytics)
- [Tee: Raw Output Recovery](#tee-raw-output-recovery)
- [CCR-SDK: Conversation Compression](#ccr-sdk-conversation-compression)
- [CCR-Eval: Quality Gates](#ccr-eval-quality-gates)
- [Hook Architecture](#hook-architecture)
- [CCR vs RTK](#ccr-vs-rtk)
- [Crate Overview](#crate-overview)

---

## Why CCR

Every time Claude Code runs a shell command, it reads the full output. A single `cargo build` with 800 lines of `Compiling …` noise wastes thousands of tokens. Multiply that across a coding session and you're paying for output that carries zero signal.

CCR solves this in two layers:

| Layer | Mechanism | Coverage |
|-------|-----------|----------|
| **PreToolUse** | `ccr-rewrite.sh` intercepts every Bash call before execution; wraps known commands in `ccr run` | 9 specialized handlers |
| **PostToolUse** | `ccr hook` receives the output after any non-rewritten command; runs the full pipeline | Every other command |

**CCR's moat over rule-based proxies:**

- **BERT semantic compression** — Lines are scored by distance from the centroid of the output. Repetitive noise clusters near the centroid and gets dropped. Errors and unique events are outliers and get kept. This works on output that regex rules will never anticipate.
- **Docker BERT dedup** — "connection refused to 10.0.0.1" and "connection refused to 10.0.0.2" are semantically identical. CCR collapses them into one representative line. Rule-based tools treat them as different.
- **Smart `cat`** — For files over 500 lines, CCR uses BERT importance scoring rather than head+tail. You get the structurally important lines, not just the first and last ones.
- **Conversation compression** (ccr-sdk) — 10–20% cumulative savings per turn by compressing old turns in the conversation history. Compounds across a long session.

---

## How It Works

```
Claude Code issues: git status
        │
        ▼ PreToolUse hook (ccr-rewrite.sh)
        │  - Reads tool_input.command
        │  - Calls `ccr rewrite "git status"`
        │  - git is a known handler → returns `ccr run git status`
        │  - Hook patches tool_input.command in place
        ▼
Claude Code executes: ccr run git status
        │
        ▼ ccr run
        │  - Looks up GitHandler
        │  - rewrite_args: injects --oneline if not present
        │  - Executes: git status
        │  - Captures stdout + stderr combined
        │  - Writes raw output to ~/.local/share/ccr/tee/<ts>_git.log
        │  - GitHandler.filter() → compact changed-file list
        │  - Appends analytics record to analytics.jsonl
        │  - Prints filtered output; appends "[full output: ...]" if >60% savings
        ▼
Claude reads: compact output (80% fewer tokens)

──────────────────────────────────────────────────────────

Claude Code issues: some-unknown-tool --flag
        │
        ▼ PreToolUse hook
        │  - ccr rewrite "some-unknown-tool --flag" → exits 1 (no handler)
        │  - Hook passes command through unmodified
        ▼
Claude Code executes: some-unknown-tool --flag  (raw)
        │
        ▼ PostToolUse hook (ccr hook)
        │  - Receives { tool_response: { output: "..." } }
        │  - Runs full pipeline:
        │      strip ANSI → normalize whitespace → apply patterns → BERT summarize
        │  - Returns { output: "<filtered>" }
        ▼
Claude reads: BERT-compressed output (~40% savings on unknown commands)
```

---

## Installation

### Prerequisites

- Rust toolchain (`rustup`, stable)
- `jq` (for the PreToolUse shell hook)

### Build from source

```bash
git clone <repo>
cd ccr
cargo build --release
```

Copy the binary to your PATH:

```bash
cp target/release/ccr ~/.local/bin/
# or
sudo cp target/release/ccr /usr/local/bin/
```

### Register hooks with Claude Code

```bash
ccr init
```

This writes two entries into `~/.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [{ "type": "command", "command": "/path/to/ccr-rewrite.sh" }]
    }],
    "PostToolUse": [{
      "matcher": "Bash",
      "hooks": [{ "type": "command", "command": "ccr hook" }]
    }]
  }
}
```

It also writes `~/.claude/hooks/ccr-rewrite.sh` and makes it executable.

### Verify

```bash
ccr gain          # shows analytics (zeros on first run)
ccr run git status  # should print compact output, not raw git output
```

---

## Quick Start

After `ccr init`, everything is automatic. Claude Code's Bash calls for known commands are silently rewritten to `ccr run <cmd>`. You don't change how you work.

To see savings accumulate:

```bash
# After a coding session:
ccr gain

# See daily breakdown over the last 14 days:
ccr gain --history

# Find commands you're running raw (not yet covered by CCR):
ccr discover
```

To test a handler directly:

```bash
ccr run cargo build 2>&1
ccr run git log -n 10
ccr run ls -la src/
```

To run a command without filtering (but still record analytics):

```bash
ccr proxy cargo build 2>&1
```

---

## Commands

### ccr run

Execute a command through CCR's handler pipeline.

```
ccr run <command> [args...]
```

**What it does:**

1. Extracts `argv[0]` as the handler key.
2. If a handler exists: calls `handler.rewrite_args(args)` to potentially inject flags (e.g. `--message-format json` for cargo).
3. Executes the (possibly rewritten) command, capturing stdout + stderr combined.
4. Writes raw output to `~/.local/share/ccr/tee/<timestamp>_<cmd>.log`.
5. Calls `handler.filter(raw_output, original_args)` to produce compact output.
6. If no handler: falls through to the full BERT pipeline.
7. Appends a hint `[full output: ~/.local/share/ccr/tee/...]` when savings exceed 60%.
8. Records `{ command, subcommand, input_tokens, output_tokens, duration_ms }` to `analytics.jsonl`.
9. Propagates the original exit code.

**Examples:**

```bash
ccr run cargo test -p ccr-core
ccr run git diff HEAD~1
ccr run curl -s https://httpbin.org/json
ccr run docker logs my-container --tail 100
```

**Compound commands** are handled by the PreToolUse hook during rewriting (see [Hook Architecture](#hook-architecture)), not by `ccr run` itself.

---

### ccr gain

Show token savings analytics.

```
ccr gain [--history] [--days N]
```

**Default view:**

```
CCR Token Savings
═════════════════════════════════════════════════
  Runs:           142
  Tokens saved:   182.2k  (77.7%)
  Cost saved:     ~$0.547  (at $3.00/1M input tokens)
  Today:          23 runs · 31.4k saved · 74.3%
  7-day:          98 runs · 126.8k saved · 76.8%

Per-Command Breakdown
─────────────────────────────────────────────────────────────────
COMMAND        RUNS       SAVED   SAVINGS   AVG ms  IMPACT
─────────────────────────────────────────────────────────────────
cargo            45       89.2k     87.2%      420  ████████████████████
git              31       41.1k     79.1%       82  ████████████████
curl             12       31.2k     94.3%      210  ██████████████████
npm               8       12.1k     85.1%     1240  ████████████████
(pipeline)       18       12.4k     42.1%        —  ████████
```

- **SAVED** — absolute tokens eliminated (formatted as k/M)
- **SAVINGS** — percentage of input tokens removed
- **AVG ms** — average wall-clock execution time of the underlying command
- **IMPACT** — bar scaled to 100% = 20 blocks (each block = 5%)
- **(pipeline)** — commands that went through the BERT pipeline (no specialized handler)

**History view (`--history`):**

```bash
ccr gain --history          # last 14 days (default)
ccr gain --history --days 7 # last 7 days
```

```
CCR Daily History  (last 14 days)
════════════════════════════════════════════════════════════
DATE          RUNS        SAVED   SAVINGS   COST SAVED
────────────────────────────────────────────────────────────
2026-03-17      23        31.4k     74.3%       $0.094
2026-03-16      41        58.1k     78.1%       $0.174
2026-03-15      28        40.2k     71.2%       $0.121
...
────────────────────────────────────────────────────────────
14-day total   182       182.2k     77.7%       $0.547

Top Commands
──────────────────────────────────────────
COMMAND          RUNS       SAVED   SAVINGS
──────────────────────────────────────────
cargo              45       89.2k    87.2%
git                31       41.1k    79.1%
curl               12       31.2k    94.3%
```

Cost is estimated at **$3.00 / 1M input tokens** (Claude Sonnet 4.6 input price). Tool output becomes Claude's input, so this directly maps to your API bill.

---

### ccr discover

Scan Claude Code session history for commands you're running without CCR.

```
ccr discover
```

Reads all JSONL files under `~/.claude/projects/*/`, extracts Bash tool calls that are **not** already wrapped in `ccr run`, and reports estimated savings per command.

**Example output:**

```
CCR Discover — Missed Savings
==============================
COMMAND       CALLS       OUTPUT   SAVINGS  IMPACT
────────────────────────────────────────────────────────
cargo           120       4.2MB     87.0%  ████████████████████
git              89       1.8MB     80.0%  ████████████████
docker           34       2.1MB     85.0%  █████████████████
curl             21       0.9MB     96.0%  ████████████████████
ls               67       0.4MB     80.0%  ████████████████
────────────────────────────────────────────────────────
Potential savings: 7.6MB bytes across 5 command types

Run `ccr init` to enable PreToolUse auto-rewriting.
```

Savings estimates use per-handler expected rates (e.g. 96% for `curl`, 87% for `cargo`). Unknown commands default to the BERT pipeline estimate (40%).

---

### ccr init

Install CCR hooks into `~/.claude/settings.json`.

```
ccr init
```

1. Writes `~/.claude/hooks/ccr-rewrite.sh` (the PreToolUse bash script) and `chmod +x`s it.
2. Registers both `PreToolUse` and `PostToolUse` hook entries in `~/.claude/settings.json`.
3. Creates `~/.claude/` and subdirectories if they don't exist.

Running `ccr init` is idempotent — re-running it overwrites the hook entries cleanly.

**What gets written to settings.json:**

```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [{
        "type": "command",
        "command": "~/.claude/hooks/ccr-rewrite.sh"
      }]
    }],
    "PostToolUse": [{
      "matcher": "Bash",
      "hooks": [{ "type": "command", "command": "ccr hook" }]
    }]
  }
}
```

**What `ccr-rewrite.sh` does:**

```bash
#!/usr/bin/env bash
INPUT=$(cat)
CMD=$(echo "$INPUT" | jq -r '.tool_input.command // empty')
[ -z "$CMD" ] && exit 0
REWRITTEN=$(ccr rewrite "$CMD" 2>/dev/null) || exit 0
[ "$CMD" = "$REWRITTEN" ] && exit 0
ORIGINAL_INPUT=$(echo "$INPUT" | jq -c '.tool_input')
UPDATED_INPUT=$(echo "$ORIGINAL_INPUT" | jq --arg cmd "$REWRITTEN" '.command = $cmd')
jq -n --argjson updated "$UPDATED_INPUT" \
  '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow",
    "permissionDecisionReason":"CCR auto-rewrite","updatedInput":$updated}}'
```

If `ccr rewrite` exits 1 (no handler matched), the hook exits 0 without emitting JSON, and Claude Code proceeds with the original command unmodified.

---

### ccr filter

Read from stdin, apply the pipeline, write to stdout.

```
ccr filter [--command <hint>]
```

**Options:**

- `--command <hint>` — Pass the command name to select matching patterns (e.g. `--command cargo`). Without this, only global rules apply.

**Use cases:**

```bash
# Pipe arbitrary output through the pipeline
cargo clippy 2>&1 | ccr filter --command cargo

# Filter a file
cat large_log.txt | ccr filter

# Compose with other tools
kubectl logs my-pod | ccr filter
```

**Pipeline stages (in order):**

1. Strip ANSI escape sequences (if `strip_ansi = true`)
2. Normalize whitespace (trim trailing spaces, deduplicate consecutive identical lines, collapse multiple blank lines to one)
3. Apply per-command regex patterns (Remove / Collapse / ReplaceWith)
4. If output exceeds `summarize_threshold_lines`: BERT semantic summarization

After writing to stdout, appends an analytics record to `analytics.jsonl`.

---

### ccr proxy

Execute a command with no filtering. Records analytics as a baseline.

```
ccr proxy <command> [args...]
```

Useful for debugging handlers ("what does the raw output look like?") or establishing a performance baseline. The raw output is printed to stdout/stderr and also written to a tee file with `_proxy` suffix.

Since no compression is applied, the analytics record will show `savings_pct = 0.0`. You can compare this against a subsequent `ccr run` call to verify the handler is working.

```bash
# See raw cargo output
ccr proxy cargo build 2>&1

# Compare token counts
ccr gain   # check the (pipeline) and cargo rows after both calls
```

---

## Handlers

Handlers live in `ccr/src/handlers/`. Each implements two methods:

```rust
fn rewrite_args(&self, args: &[String]) -> Vec<String>   // optional: inject flags
fn filter(&self, output: &str, args: &[String]) -> String // required: compact output
```

Unknown commands (no handler match) fall through to the [Pipeline](#pipeline-unknown-commands).

---

### cargo

**Savings: ~87%**

Handles `cargo build`, `cargo check`, `cargo clippy`, and `cargo test`.

#### build / check / clippy

**rewrite_args:** Injects `--message-format json` unless already present.

**filter:** Parses the JSON compiler message stream. For each line:

- `"reason": "compiler-artifact"` — silently discarded (pure noise)
- `"reason": "compiler-message"` with `"level": "error"` — kept verbatim with file+line location
- `"reason": "compiler-message"` with `"level": "warning"` — collected; first 3 shown inline, rest summarized as `[+N more warnings]`
- `"reason": "build-finished"` — extracts the `success` boolean

**Output examples:**

```
# Success with warnings:
[3 warnings]
  warning: unused variable `x` [src/main.rs:12]
  warning: dead_code: function `foo` [src/lib.rs:45]
  warning: unused import: `std::fmt` [src/lib.rs:1]
  [+12 more warnings]
Build OK

# Failure:
error: expected `;`, found `}` [src/main.rs:42]
error[E0308]: mismatched types [src/lib.rs:88]
[1 warning]
  warning: unused variable `x` [src/main.rs:12]
```

Non-JSON lines (e.g. mixed stderr output without the JSON flag) are handled with a regex fallback: lines starting with `error` or `warning` are kept; everything else is dropped.

#### test

**rewrite_args:** No modification (cargo test JSON output requires nightly; CCR parses the stable text format instead).

**filter:** Parses the standard `cargo test` output:

- Lines matching `test <name> ... FAILED` → collected as failure names
- `failures:` section → kept (up to 20 lines of failure detail)
- `test result:` line → always kept as the summary

**Output examples:**

```
# All passed:
test result: ok. 40 passed; 0 failed; 0 ignored

# Failures:
FAILED: tests::test_parser
FAILED: tests::test_edge_case

---- tests::test_parser stdout ----
thread 'tests::test_parser' panicked at 'assertion failed', src/lib.rs:23

test result: FAILED. 38 passed; 2 failed; 0 ignored
```

---

### git

**Savings: ~80%**

**rewrite_args:** Injects `--oneline` into `git log` calls unless already present.

Per-subcommand filters:

#### status

Drops help-text lines (`(use "git …"`, `nothing to commit`, `no changes added to commit`) and the blank lines around them. If more than 20 files are changed, shows the first 20 and appends `[+N more files]`. On a clean tree returns `nothing to commit, working tree clean`.

```
On branch main
Your branch is up to date with 'origin/main'.

Changes not staged for commit:
	modified:   src/main.rs
	modified:   src/handlers/git.rs
```
→
```
On branch main
	modified:   src/main.rs
	modified:   src/handlers/git.rs
```

#### log

With `--oneline` injected, limits to 20 entries and appends `[+N more commits]` for longer histories.

#### diff

Keeps only structurally meaningful lines:
- `diff --git …` — file header
- `index …` — blob hashes
- `--- a/…` / `+++ b/…` — file paths
- `@@ … @@` — hunk headers
- `+…` / `-…` — actual changes

Context lines (lines starting with a space) are dropped entirely.

#### push / pull / fetch

Drops progress noise (`Counting objects`, `Compressing objects`, `Writing objects`, `Delta compression`, `remote: Enumerating`) and keeps only meaningful lines such as branch tracking info, fast-forward notifications, and summary counts.

#### commit / add

Keeps the one-liner summary (e.g. `[main 3a7f2c1] fix: correct off-by-one`) and the `N file changed, M insertions(+)` line.

#### branch / stash

Lists up to 30 entries; appends `[+N more]` for longer lists.

---

### curl

**Savings: ~96% on JSON APIs**

#### JSON responses

Detects JSON either by `Content-Type: application/json` in response headers (when using `curl -i`) or by a `{` / `[` prefix on the body.

Replaces every value with its type descriptor:

| Original | Schema |
|----------|--------|
| `"name": "Alice"` | `"name": "string"` |
| `"count": 42` | `"count": "number"` |
| `"active": true` | `"active": "boolean"` |
| `"data": null` | `"data": "null"` |
| `"tags": ["a","b","c"]` | `"tags": [{"first element schema"}, "[3 items total]"]` |

Arrays show the schema of the **first element** plus an item count — giving you the shape without repeating it for every item.

**Size guard:** If the derived schema is larger than the original JSON, CCR passes the original through unchanged. This prevents CCR from making things worse on tiny responses.

#### Non-JSON responses

Passed through the BERT pipeline fallback.

**Example:**

```json
{
  "id": 1,
  "user": {
    "name": "Alice",
    "email": "alice@example.com",
    "roles": ["admin", "user"]
  },
  "items": [
    {"id": 1, "title": "First", "price": 9.99},
    {"id": 2, "title": "Second", "price": 14.99}
  ],
  "total": 24.98
}
```
→
```json
{
  "id": "number",
  "user": {
    "name": "string",
    "email": "string",
    "roles": ["string", "[2 items total]"]
  },
  "items": [
    {"id": "number", "title": "string", "price": "number"},
    "[2 items total]"
  ],
  "total": "number"
}
```

---

### docker

**Savings: ~85% — with CCR-unique BERT semantic dedup**

#### logs

**rewrite_args:** Appends `--tail 200` unless `--tail` is already specified.

**filter:** Applies **BERT semantic deduplication** — the key differentiator from other tools.

The algorithm:
1. Embed all non-empty lines using `fastembed::AllMiniLML6V2` (384-dim vectors).
2. For each line (in order): check cosine similarity against every already-kept line.
3. If similarity > **0.90** to any kept line → it's semantically duplicate, drop it.
4. Hard-keep lines containing `error`, `panic`, `fatal`, `exception`, `failed`, `stack trace`, `caused by`, `at ` (stack frame prefix).
5. Falls back to exact-match dedup if the embedding model is unavailable.

**Why this beats regex dedup:**

```
# Raw docker logs (50 similar lines):
2026-03-17 10:01:02 connection refused to 10.0.0.1:5432
2026-03-17 10:01:03 connection refused to 10.0.0.2:5432
2026-03-17 10:01:04 connection refused to 10.0.0.3:5432
...

# Exact-match dedup: keeps all 50 (different IPs = different strings)
# CCR BERT dedup: keeps 1 representative + "[49 duplicate lines collapsed]"
```

#### ps

Extracts container name, status, and ports. Drops container ID, image, command, and created-at columns.

#### images

Extracts repository, tag, and size. Drops image ID, created-at, and `VIRTUAL SIZE` duplicates.

#### compose ps / compose logs

`docker-compose` is registered as an alias and receives the same treatment.

---

### npm / pnpm / yarn

**Savings: ~85%**

All three package managers are handled identically (handler key: `npm`, `pnpm`, `yarn`).

#### install / add / ci

Replaces the entire install output with a single summary line:

```
[install complete — 342 packages]
```

Preserves any audit/vulnerability lines if present.

#### test

Parses test runner output (Jest, Vitest, Mocha):

- Failure lines (`✕`, `✗`, `× `, `FAIL `) → kept
- Bullet failure details starting with `●` → kept (up to blank-line delimiter)
- Final summary containing `passing` / `failing` / `passed` / `failed` → kept

On a full pass, returns only the summary line.

#### run \<script>

For build scripts and other `npm run` invocations:
- If output ≤ 30 lines: pass through
- If > 30 lines: keep error/warning/success/done/built lines + last 5 lines + total line count

---

### ls

**Savings: ~80%**

Detects `ls -l` format (lines starting with permission bits or `total N`) vs bare listing format.

**Behavior:**
- Sorts entries: directories first (alphabetical), then files (alphabetical)
- Limits to 40 entries; appends `[+N more]` beyond that
- Appends a summary: `[3 dirs, 12 files]`
- Directory names get a trailing `/`

```
Makefile
README.md
ccr/
ccr-core/
ccr-eval/
ccr-sdk/
config/
target/
```
→
```
ccr/
ccr-core/
ccr-eval/
ccr-sdk/
config/
target/
Cargo.lock
Cargo.toml
Makefile
README.md
[4 dirs, 4 files]
```

---

### cat

**Savings: ~70% on large files — with BERT for files >500 lines**

The `cat` command is registered under handler key `"cat"`.

| File size | Strategy |
|-----------|----------|
| ≤ 100 lines | Pass through unchanged |
| 101–500 lines | Head 60 lines + `[... N lines omitted ...]` + tail 20 lines |
| > 500 lines | BERT semantic summarization with budget of 80 lines |

For the BERT path, CCR uses the same centroid-scoring algorithm as the pipeline: each line is scored by distance from the output's semantic centroid, and the 80 highest-scoring (most unusual/informative) lines are kept. Error and warning lines are hard-kept regardless of score. Omission markers show how many lines were dropped between each kept segment.

This means CCR keeps the *interesting* lines from a large file — errors, unique patterns, structural landmarks — not just the first and last N.

---

### grep / rg

**Savings: ~80%**

Groups matches by filename and truncates long lines.

**Detection:** Looks for `file:content` or `file:lineno:content` format. If the filename portion contains no spaces and starts a colon-delimited field, it's treated as a file group.

**Behavior:**
- Groups matches under their source filename
- Truncates each match line to **120 characters** (appends `…`)
- Limits total to **50 matches** across all files
- Appends `[+N more in M files]` when truncated

**Output:**

```
src/handlers/cargo.rs:
  fn filter_build(output: &str) -> String {
  fn filter_test(output: &str) -> String {
src/handlers/git.rs:
  fn filter_status(output: &str) -> String {
  fn filter_log(output: &str) -> String {
[+12 more in 4 files]
```

---

### find

**Savings: ~78%**

Detects the common path prefix across all results, strips it, and groups remaining paths by parent directory.

**Behavior:**
- Strips common prefix (trimmed to last `/`)
- Groups by immediate parent directory
- Shows up to 5 filenames per directory inline; appends `[+N more]` for larger directories
- Limits total displayed entries to **50**
- Appends `[N total, M dirs]` summary

**Output:**

```
# find . -name "*.rs" (500 results)
[root: /Users/user/Desktop/ccr]

ccr/src/ (12 entries)
  main.rs
  hook.rs
  config_loader.rs
  [+9 more]
ccr-core/src/ (11 entries)
  lib.rs
  analytics.rs
  pipeline.rs
  [+8 more]
[500 total, 18 dirs]
```

---

## Pipeline (Unknown Commands)

Any command without a registered handler — whether via `ccr run` fallback or the PostToolUse hook — goes through the four-stage pipeline defined in `ccr-core`.

### Stage 1: Strip ANSI

Removes all ANSI escape sequences (color codes, cursor movement, terminal control). Controlled by `strip_ansi = true` in config. Uses a regex covering:
- SGR sequences: `ESC [ <params> m`
- Cursor movement: `ESC [ <params> [A-Z]`
- Character set designations: `ESC ( <code>`

### Stage 2: Normalize Whitespace

- Trims trailing spaces from every line
- Removes consecutive duplicate lines (same string repeated)
- Collapses runs of blank lines to a single blank line

### Stage 3: Apply Regex Patterns

Loads per-command patterns from config (matched by command hint). Each pattern specifies a regex and an action:

| Action | Effect |
|--------|--------|
| `Remove` | Delete the matching line |
| `Collapse` | Accumulate consecutive matching lines; emit `[N matching lines collapsed]` when run ends |
| `ReplaceWith("text")` | Replace the matching line with a static string |

Patterns are tested in order; the first match wins for each line.

**Built-in patterns** (`config/default_filters.toml`):

```toml
[commands.git]
patterns = [
  { regex = "^(Counting|Compressing|Receiving|Resolving) objects:.*", action = "Remove" },
  { regex = "^remote: (Counting|Compressing|Enumerating).*", action = "Remove" },
]

[commands.cargo]
patterns = [
  { regex = "^\\s+Compiling \\S+ v[\\d.]+", action = "Collapse" },
  { regex = "^\\s+Downloaded \\S+ v[\\d.]+", action = "Remove" },
]

[commands.npm]
patterns = [
  { regex = "^npm warn.*", action = "Remove" },
  { regex = "^added \\d+ packages.*", action = { ReplaceWith = "[npm install complete]" } },
  { regex = "^\\d+ packages are looking for funding.*", action = "Remove" },
]

[commands.docker]
patterns = [
  { regex = "^ ---> [a-f0-9]+$", action = "Remove" },
  { regex = "^Removing intermediate container.*", action = "Remove" },
]
```

### Stage 4: BERT Semantic Summarization

Triggered when line count exceeds `summarize_threshold_lines` (default: 200).

**Algorithm:**

1. Embed all non-blank lines using `fastembed::AllMiniLML6V2` (384-dim).
2. Compute centroid of all embeddings.
3. Score each line as `1 - cosine_similarity(embedding, centroid)`. High score = outlier = informative.
4. Hard-keep any line matching `/(error|warning|warn|failed|failure|fatal|panic|exception|critical)/i`.
5. Sort remaining lines by score descending. Accept lines scoring above **40% of the max score** until the budget (`head_lines + tail_lines = 60`) is filled.
6. Reconstruct in original order, inserting `[... N lines omitted ...]` markers at each gap.

**Fallback:** If the embedding model fails (download unavailable, OOM, etc.), CCR falls back to deterministic head+tail: first `head_lines` lines + `[... N lines omitted ...]` + last `tail_lines` lines.

The BERT model (`AllMiniLML6V2`, ~22MB) is downloaded on first use via `fastembed`'s automatic model cache.

---

## Configuration

CCR loads configuration from the first file found in this order:

1. `./ccr.toml` (current directory)
2. `~/.config/ccr/config.toml`
3. Embedded default (`config/default_filters.toml`, compiled into the binary)

### Full schema

```toml
[global]
# Lines above this threshold trigger BERT semantic summarization
summarize_threshold_lines = 200

# Head+tail line budget for the summarization fallback
head_lines = 30
tail_lines = 30

# Strip ANSI color/cursor escape sequences from all output
strip_ansi = true

# Trim trailing spaces, deduplicate consecutive identical lines,
# collapse multiple blank lines to one
normalize_whitespace = true

# Remove consecutive identical lines (subset of normalize_whitespace)
deduplicate_lines = true


[tee]
# Write raw command output to disk before filtering
enabled = true

# "aggressive" = only when savings > 60%
# "always"     = every ccr run invocation
# "never"       = disabled
mode = "aggressive"

# Maximum tee files to keep; oldest deleted on overflow
max_files = 20


# Per-command pattern rules.
# Command hint comes from argv[0] of the shell command.
[commands.git]
patterns = [
  { regex = "^(Counting|Compressing|Receiving|Resolving) objects:.*", action = "Remove" },
  { regex = "^remote: (Counting|Compressing|Enumerating).*",          action = "Remove" },
]

[commands.cargo]
patterns = [
  { regex = "^\\s+Compiling \\S+ v[\\d.]+", action = "Collapse" },
  { regex = "^\\s+Downloaded \\S+ v[\\d.]+", action = "Remove"   },
]

[commands.npm]
patterns = [
  { regex = "^npm warn.*",                          action = "Remove"                            },
  { regex = "^added \\d+ packages.*",               action = { ReplaceWith = "[npm install complete]" } },
  { regex = "^\\d+ packages are looking for funding.*", action = "Remove"                       },
]

[commands.docker]
patterns = [
  { regex = "^ ---> [a-f0-9]+$",               action = "Remove" },
  { regex = "^Removing intermediate container.*", action = "Remove" },
]

# Add your own:
[commands.kubectl]
patterns = [
  { regex = "^Warning:.*", action = "Remove" },
]
```

### Adding a custom command handler

For regex-based filtering, add a `[commands.<name>]` section to `~/config/ccr/config.toml`. For more complex filtering, implement the `Handler` trait in `ccr/src/handlers/`:

```rust
pub struct MyToolHandler;

impl Handler for MyToolHandler {
    fn rewrite_args(&self, args: &[String]) -> Vec<String> {
        // optionally inject flags
        args.to_vec()
    }

    fn filter(&self, output: &str, args: &[String]) -> String {
        // return compact output
        output.to_string()
    }
}
```

Register it in `get_handler()` in `ccr/src/handlers/mod.rs`:

```rust
"my-tool" => Some(Box::new(my_tool::MyToolHandler)),
```

---

## Analytics

All CCR operations write a record to `~/.local/share/ccr/analytics.jsonl`.

### Record schema

```json
{
  "input_tokens": 4821,
  "output_tokens": 612,
  "savings_pct": 87.3,
  "command": "cargo",
  "subcommand": "build",
  "timestamp_secs": 1742198400,
  "duration_ms": 3420
}
```

| Field | Description |
|-------|-------------|
| `input_tokens` | Token count of raw command output (before filtering) |
| `output_tokens` | Token count of filtered output |
| `savings_pct` | `(input - output) / input × 100` |
| `command` | `argv[0]` of the executed command |
| `subcommand` | `argv[1]` if it's not a flag (e.g. `"build"`, `"status"`, `"logs"`) |
| `timestamp_secs` | Unix timestamp when the record was written |
| `duration_ms` | Wall-clock execution time of the underlying command |

`command` and `subcommand` are `null` for pipeline-only runs (PostToolUse hook, `ccr filter`). `duration_ms` is `null` for those paths too. All new fields are `#[serde(default)]` for backward compatibility with old records.

### Storage

- **Location:** `~/.local/share/ccr/analytics.jsonl`
- **Format:** One JSON object per line, append-only
- **No rotation** — the file grows indefinitely (it's text, so stays small even after millions of runs)

---

## Tee: Raw Output Recovery

Every `ccr run` invocation saves the unfiltered output before filtering. This solves the trust problem: if CCR's handler drops something you needed, you can recover it without re-running the command.

### Location

```
~/.local/share/ccr/tee/<timestamp>_<command>.log
~/.local/share/ccr/tee/<timestamp>_<command>_proxy.log  (for ccr proxy)
```

### Recovery hint

When savings exceed 60%, the filtered output includes a recovery line:

```
error: mismatched types [src/main.rs:42]
[full output: ~/.local/share/ccr/tee/1742198400_cargo.log]
```

Claude can `cat` that path without re-executing the command.

### Rotation

CCR keeps at most `max_files` tee files (default: 20). When a new file would exceed the limit, the oldest files are deleted first.

### Modes

| Mode | When tee file is written |
|------|--------------------------|
| `aggressive` | Only when savings_pct > 60% (default) |
| `always` | Every `ccr run` invocation |
| `never` | Disabled |

Configure in `ccr.toml`:

```toml
[tee]
mode = "always"
max_files = 50
```

---

## CCR-SDK: Conversation Compression

The `ccr-sdk` crate compresses old turns in the Claude Code conversation history. This is orthogonal to per-command token reduction — it compounds across the entire session.

**Typical savings: 10–20% cumulative per turn**, growing as the session extends.

### Architecture

```
messages (oldest to newest):
  [tier 2][tier 2][tier 2][tier 1][tier 1][verbatim][verbatim][verbatim]
   ←──────────────────────────────────── age ───────────────────────────→
```

| Tier | Default age | Compression |
|------|-------------|-------------|
| Verbatim | 0–2 (most recent) | No change |
| Tier 1 | 3–7 | Extractive: keep 55% of sentences |
| Tier 2 | 8+ | Generative (Ollama) if available; extractive 20% otherwise |

### Compression config

```rust
CompressionConfig {
    recent_n: 3,               // messages kept verbatim
    tier1_n: 5,                // messages in tier 1
    tier1_ratio: 0.55,         // keep 55% of sentences
    tier2_ratio: 0.20,         // extractive fallback: keep 20%
    tier2_assistant_ratio: 0.60, // assistant messages in tier 2: keep 60%
    ollama: Some(OllamaConfig {
        base_url: "http://localhost:11434",
        model: "mistral:instruct",
        similarity_threshold: 0.80,  // BERT quality gate
    }),
    max_context_tokens: Some(80_000),  // budget enforcement
}
```

### Sentence selection (extractive)

Sentences are scored using the same BERT centroid method as the pipeline. Hard-keep rules differ by role:

**User messages:** Questions (`?`), code-bearing sentences (backticks, `::`), snake_case identifiers, constraint language (`must`, `never`, `always`, `ensure`, `do not`, `don't`, `avoid`, `required`, `critical`).

**Assistant messages:** Code-bearing sentences, list items (`-`, `*`, numbered), currency/percentage values, any sentence containing a number (data points, dates, counts), constraint language.

### Generative compression (Ollama)

When Ollama is reachable (health check with 2-second timeout), tier-2 user messages are compressed generatively:

1. Prompt: `"Compress the following to ~60% word count. Preserve every fact, name, and constraint. …"`
2. **BERT quality gate:** If `cosine_similarity(original, generated) < 0.80` → reject generated output and fall back to extractive.
3. On any Ollama error: fall back to extractive silently.

### Semantic deduplication

The `Deduplicator` scans user messages for sentences that repeat content already present in older turns:

- Threshold: cosine similarity > **0.92**
- Replacement: `[covered in turn N]`
- Assistant messages are never modified
- Operates oldest-first to preserve the canonical version

### Budget enforcement

If `max_context_tokens` is set, a second pass runs after tier assignment:

1. Compress user messages oldest-first until under budget.
2. If still over budget: compress assistant messages.

### Usage

```rust
use ccr_sdk::{Compressor, CompressionConfig, Message, Role};

let messages = vec![
    Message { role: Role::User, content: "...".to_string() },
    Message { role: Role::Assistant, content: "...".to_string() },
    // ...
];

let config = CompressionConfig::default();
let compressor = Compressor::new(config);
let result = compressor.compress(messages)?;

println!("Saved {} tokens ({:.1}%)",
    result.tokens_in - result.tokens_out,
    (result.tokens_in - result.tokens_out) as f32 / result.tokens_in as f32 * 100.0
);
```

---

## CCR-Eval: Quality Gates

The `ccr-eval` crate provides an evaluation suite to ensure compression doesn't lose important information.

### Command output fixtures

Each fixture is a pair:
- `<name>.txt` — raw command output
- `<name>.qa.toml` — Q&A assertions

```toml
# fixtures/cargo_error.qa.toml
[[questions]]
question = "What file contains the type mismatch error?"
expected_contains = ["src/main.rs"]

[[questions]]
question = "What is the error code?"
expected_contains = ["E0308"]
```

The runner compresses the `.txt` through CCR's pipeline, then queries Claude via the API to answer each question from the compressed output. A pass requires the expected terms to appear in the answer.

### Conversation compression fixtures

```toml
# fixtures/code_review.conv.toml
[[turns]]
role = "user"
content = "Please review this function for potential bugs: ..."

[[turns]]
role = "assistant"
content = "I see three potential issues: ..."
```

The runner measures:
- V1 (BERT extractive only) vs V2 (Ollama + BERT gate) token counts
- BERT semantic similarity between compressed and original
- Information retention score

### Running evaluations

```bash
# Build ccr-eval
cargo build -p ccr-eval --release

# Run against all fixtures (requires ANTHROPIC_API_KEY)
ANTHROPIC_API_KEY=sk-... ./target/release/ccr-eval

# Run conversation fixtures only
./target/release/ccr-eval --conv
```

---

## Hook Architecture

### PreToolUse (command rewriting)

Triggered **before** the Bash tool executes. CCR patches `tool_input.command` in the hook JSON and Claude Code uses the patched value.

**Compound command handling:** `ccr rewrite` splits on `&&`, `||`, and `;` and rewrites each segment independently:

```bash
# Input to ccr rewrite:
cargo build && git push origin main && npm test

# ccr rewrite output:
ccr run cargo build && ccr run git push origin main && ccr run npm test
```

Segments with no handler match are left unchanged:
```bash
my-tool --flag && git status
→ my-tool --flag && ccr run git status
```

**Double-wrap protection:** If a command already starts with `ccr run`, `ccr rewrite` exits 1 without output. The hook treats exit 1 as "pass through unchanged."

**Full PreToolUse hook JSON contract:**

```json
// Input (from Claude Code):
{
  "tool_name": "Bash",
  "tool_input": { "command": "cargo build" }
}

// Output (from ccr-rewrite.sh, when rewriting):
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow",
    "permissionDecisionReason": "CCR auto-rewrite",
    "updatedInput": { "command": "ccr run cargo build" }
  }
}

// Output (when no rewrite needed): nothing — hook exits 0 with no output
```

### PostToolUse (output filtering)

Triggered **after** every Bash tool execution. Receives the full output and returns a replacement.

**Input JSON:**

```json
{
  "tool_name": "Bash",
  "tool_input": { "command": "some-tool --flag" },
  "tool_response": {
    "output": "lots of raw output...",
    "error": null
  }
}
```

**Output JSON:**

```json
{ "output": "filtered output" }
```

**Graceful degradation:** The hook never fails. On malformed JSON, parse errors, empty output, or pipeline errors, `ccr hook` returns `Ok(())` with no output — Claude Code sees the original response unchanged.

**Command hint extraction:** The first word of `tool_input.command` is used as the command hint for pattern selection (e.g. `"cargo"` from `"cargo build --release"`).

### Hook execution flow summary

```
Claude Code (PreToolUse)  →  ccr-rewrite.sh  →  ccr rewrite "<cmd>"
  ↓ if known command                              ↓
  patch command in place      ←──── "ccr run <cmd>"
  run patched command: ccr run <cmd>
  ccr run executes <cmd>, filters, prints

Claude Code (PostToolUse)  →  ccr hook
  ↓ for any Bash tool call     ↓
  replace output              pipeline.process(output)  →  BERT  →  { "output": "..." }
```

---

## CCR vs RTK

| Feature | CCR | RTK |
|---------|-----|-----|
| Architecture | PreToolUse rewrite + PostToolUse filter | PreToolUse rewrite only |
| Unknown commands | BERT semantic compression (~40% savings) | Pass through unfiltered (0%) |
| BERT inside handlers | `docker logs`, `cat` | — |
| Semantic log dedup | `docker logs` (cosine > 0.90) | Exact-match dedup only |
| `cat` large files | BERT importance scoring | head+tail |
| Conversation history | ccr-sdk: tiered extractive + Ollama + dedup | — |
| Cargo test | Parse test output, failures only | JSON parsing |
| curl JSON | Schema extraction with size guard | Schema extraction |
| Config | TOML (embedded + local override) | TOML |
| Tee output | Yes (rotation, recovery hints) | Yes |
| Analytics | Per-command, daily history, cost estimate | Per-command, daily history |
| `discover` | Yes (scan Claude Code history) | Yes |
| Handler count | 9 (cargo, git, curl, docker, npm, ls, cat, grep, find) | 40+ |
| Evaluation suite | ccr-eval (Q&A + conv fixtures) | — |
| Language | Rust | Rust |

**When CCR beats RTK outright:**
- Any command without a handler (BERT fallback vs zero)
- `docker logs` with varied-IP connection errors (BERT dedup vs exact-match)
- `cat` on large files (semantic importance vs head+tail)
- Long sessions (conversation compression compounds across turns)

---

## Crate Overview

```
ccr/                     Workspace root
├── ccr/                 CLI binary
│   ├── src/main.rs      Commands enum, init(), main()
│   ├── src/hook.rs      PostToolUse hook (JSON in/out)
│   ├── src/config_loader.rs  TOML discovery (local → ~/.config → embedded)
│   ├── src/cmd/
│   │   ├── filter.rs    ccr filter (stdin pipeline)
│   │   ├── run.rs       ccr run (handler dispatch, tee, analytics)
│   │   ├── proxy.rs     ccr proxy (raw execution + analytics)
│   │   ├── rewrite.rs   ccr rewrite (PreToolUse, compound splitting)
│   │   ├── gain.rs      ccr gain (summary + history views)
│   │   └── discover.rs  ccr discover (Claude Code history scan)
│   └── src/handlers/
│       ├── mod.rs       Handler trait + get_handler() registry
│       ├── cargo.rs     JSON build parsing, test output parsing
│       ├── git.rs       Per-subcommand filters
│       ├── curl.rs      JSON schema extraction
│       ├── docker.rs    BERT semantic dedup + ps/images
│       ├── npm.rs       Install summary, test failure parsing
│       ├── ls.rs        Compact listing (dirs-first)
│       ├── read.rs      Head+tail + BERT for large files
│       ├── grep.rs      File-grouped results
│       └── find.rs      Directory-grouped results
│
├── ccr-core/            Core library (no I/O)
│   ├── src/analytics.rs Analytics struct + compute methods
│   ├── src/pipeline.rs  Four-stage processing pipeline
│   ├── src/config.rs    CcrConfig, GlobalConfig, TeeConfig, FilterAction
│   ├── src/patterns.rs  PatternFilter (Remove/Collapse/ReplaceWith)
│   ├── src/summarizer.rs BERT embeddings, line-level + sentence-level summarization
│   ├── src/sentence.rs  Sentence splitter
│   ├── src/tokens.rs    tiktoken cl100k_base token counter
│   ├── src/ansi.rs      ANSI escape stripper
│   └── src/whitespace.rs Trim, dedup, blank-line collapse
│
├── ccr-sdk/             Conversation compression library
│   ├── src/compressor.rs Tiered compression + budget pass
│   ├── src/deduplicator.rs Semantic cross-turn deduplication
│   ├── src/ollama.rs    Generative summarization + BERT quality gate
│   └── src/message.rs   Message/Role types
│
├── ccr-eval/            Evaluation suite
│   ├── src/runner.rs    Fixture execution (Q&A + conv)
│   ├── src/report.rs    Results formatting
│   └── fixtures/        .qa.toml and .conv.toml test data
│
└── config/
    └── default_filters.toml   Embedded default configuration
```

### Key types

```rust
// ccr-core
pub struct Analytics { input_tokens, output_tokens, savings_pct,
                       command, subcommand, timestamp_secs, duration_ms }
pub struct CcrConfig  { global: GlobalConfig, commands: HashMap<String, CommandConfig>,
                        tee: TeeConfig }
pub struct PipelineResult { output: String, analytics: Analytics }

// ccr (handlers)
pub trait Handler { fn rewrite_args(&self, args) -> Vec<String>;
                    fn filter(&self, output, args) -> String; }
pub fn get_handler(cmd: &str) -> Option<Box<dyn Handler>>;

// ccr-sdk
pub struct CompressResult { messages: Vec<Message>, tokens_in: usize, tokens_out: usize }
pub struct CompressionConfig { recent_n, tier1_n, tier1_ratio, tier2_ratio,
                                tier2_assistant_ratio, ollama, max_context_tokens }
```
