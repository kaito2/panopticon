# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Panopticon — Intelligent AI Delegation Framework. An adaptive framework for dynamic and safe task delegation between AI agents, based on [Intelligent AI Delegation](https://arxiv.org/abs/2602.11865).

## Build & Development

```bash
cargo build              # Build
cargo test               # Run all tests (150+)
cargo clippy -- -D warnings  # Lint (must pass with zero warnings)
cargo run                # Launch the interactive REPL
```

## Architecture

Single-crate Rust project (`panopticon-ai`). The binary launches an interactive REPL (`src/repl/`) — there is no subcommand-based CLI.

Key modules:

- `src/repl/` — Interactive REPL loop (rustyline), slash command dispatch, natural language intent routing (Claude haiku), session context management, colored output
- `src/cli/` — Command handlers (`commands/`) and shared application state (`state.rs`). No clap — action enums (`TaskAction`, `AgentAction`, `ConfigAction`) are used directly by the REPL's slash dispatcher
- `src/config.rs` — TOML-based configuration (`~/.panopticon/config.toml`)
- `src/executor/` — `AgentExecutor` trait + `ClaudeExecutor` (shells out to `claude` CLI)
- `src/types/` — Core domain types: Task (11-dim characteristics, state machine), Agent, contracts, errors
- `src/decomposition/` — Task decomposition strategies (Sequential / Parallel / Hybrid)
- `src/reputation/` — EMA-based multi-dimensional reputation scoring
- `src/persistence/` — `FileStore` for JSON state persistence
- `src/ledger/` — Immutable audit ledger (in-memory + Merkle tree)
- `src/verification/` — 4 verification strategies + dispute resolution
- `src/security/` — Threat detection, circuit breakers
- `src/permissions/` — Approval levels, privilege attenuation
