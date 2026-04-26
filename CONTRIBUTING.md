# Contributing to PandaFilter

Thanks for wanting to help. PandaFilter is a Rust workspace — contributions range from adding handlers for new commands to improving the BERT-based compression pipeline.

## Setup

**Requirements:** Rust stable (1.75+), Cargo.

```bash
git clone https://github.com/AssafWoo/PandaFilter.git
cd PandaFilter
cargo build
```

The first build downloads the BERT model (~90 MB) from HuggingFace and caches it at `~/.cache/huggingface/`. Subsequent builds are instant.

## Running tests

```bash
cargo test --workspace          # all unit + integration tests
cargo test handler_benchmarks   # token-savings benchmarks (the numbers in the README)
```

## Project layout

| Crate | What it does |
|-------|-------------|
| `ccr-core` | ANSI stripping, regex filters, BERT semantic summarization, token counting |
| `ccr` (binary: `panda`) | CLI subcommands, Claude Code hook integration, analytics |
| `ccr-eval` | Evaluation framework — runs fixture pairs through the pipeline and asks Claude to verify recall |
| `ccr-sdk` | Programmatic SDK (post-MVP, mostly stubs) |

## Adding a handler for a new command

Each command (e.g. `cargo`, `pytest`, `docker`) has its own handler that knows how to compress that command's output.

1. Create `ccr/src/handlers/<yourcommand>.rs` — use `ccr/src/handlers/git.rs` as a template.
2. Implement the `Handler` trait:
   - `name()` — return the command name (e.g. `"mycommand"`)
   - `handles()` — return true if this handler should process the given command string
   - `process()` — take raw output, return filtered output
3. Register it in `ccr/src/handlers/mod.rs` — add a `mod yourcommand;` declaration and push an instance into the handler list.
4. Add a benchmark fixture in `ccr/tests/fixtures/`:
   - `mycommand.txt` — raw command output (copy from a real run)
   - `mycommand.qa.toml` — questions the agent should still be able to answer after compression
5. Run `cargo test handler_benchmarks` and verify your handler achieves meaningful savings (>40% is good, >80% is great).

## Adding TOML-based filters

For simple regex-based filtering you don't need a full Rust handler. Add rules to `config/default_filters.toml` using the 8-stage declarative pipeline:

```toml
[mycommand]
strip_ansi = true
max_lines = 50

[[mycommand.replace]]
pattern = "^Downloading.*$"
replace = ""
```

Stages (in order): `strip_ansi` → `replace` → `match_output` → `strip_lines` / `keep_lines` → `truncate_lines_at` → `head_lines` / `tail_lines` → `max_lines` → `on_empty`.

User overrides go in `.panda/filters.toml` (project) or `~/.config/panda/filters.toml` (global), which take precedence over the built-in defaults.

## PR guidelines

- **One handler per PR** — keeps reviews focused and benchmarks comparable.
- **Include a fixture** — PRs without a benchmark fixture in `ccr/tests/fixtures/` will be asked to add one.
- **Keep CI green** — run `cargo test --workspace` before pushing.
- **No unsafe code** without a clear justification.
- Open an issue first for large changes (new pipeline stages, architecture shifts) so we can discuss before you write the code.

## Questions?

Open a [GitHub Discussion](https://github.com/AssafWoo/PandaFilter/discussions) or join the [Discord](https://discord.com/invite/FFQC3bxYQ).
