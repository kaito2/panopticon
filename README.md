# Panopticon

<p align="center">
  <img src="docs/images/hero.jpg" alt="Panopticon" width="800">
</p>

Intelligent AI Delegation Framework -- an adaptive framework for dynamic and safe task delegation between AI agents.

Based on [Intelligent AI Delegation](https://arxiv.org/abs/2602.11865) (Tomasev, Franklin, Osindero — Google DeepMind, 2026).

[![Crates.io](https://img.shields.io/crates/v/panopticon-ai)](https://crates.io/crates/panopticon-ai)
[![CI](https://github.com/kaito2/panopticon/actions/workflows/ci.yml/badge.svg)](https://github.com/kaito2/panopticon/actions/workflows/ci.yml)

## Overview

Panopticon implements the five pillars of the paper:

1. **Dynamic Evaluation** -- 11-dimensional task characterization (complexity, criticality, uncertainty, verifiability, reversibility, etc.) drives all delegation decisions.
2. **Adaptive Execution** -- Event-driven coordination loop with automatic re-delegation, re-decomposition, and escalation on failures.
3. **Structural Transparency** -- Immutable audit ledger (in-memory or Merkle tree-backed) records every action across the delegation lifecycle.
4. **Scalable Market Coordination** -- RFP/bid protocol with Pareto-optimal multi-objective selection (cost, quality, latency, uncertainty, privacy risk).
5. **System Resilience** -- Circuit breakers, threat detection (Sybil, collusion, behavioral anomalies), and privilege attenuation on re-delegation.

## Quick Start

```bash
# Install from crates.io
cargo install panopticon-ai

# Launch the interactive REPL
panopticon
```

Panopticon starts an interactive REPL session. You can type natural language or use slash commands.

```
Panopticon — Intelligent AI Delegation Framework
Type natural language or /help for commands. Ctrl-D to exit.

panopticon [0 tasks, 0 agents]
> Webサイトのパフォーマンスを分析して

Planning with Claude (sonnet)...
Decomposed into 3 subtasks:
  [1] Lighthouse audit の実行 (complexity=0.4)
  [2] ボトルネック分析 (complexity=0.6)
  [3] 改善提案レポート (complexity=0.5)

Proceed? [Y/n] Y

panopticon [4 tasks, 0 agents]
> /execute --all
...
panopticon [4 tasks, 1 agents]
> /status
Tasks: 4 total (3 completed, 0 in-progress, 1 pending, 0 failed)
...
panopticon [4 tasks, 1 agents]
> /quit
State saved. Goodbye.
```

### Slash commands

| Command | Description |
|---|---|
| `/plan <goal>` | Decompose a goal into subtasks via Claude |
| `/execute [id\|--all]` | Execute tasks (by UUID or all pending) |
| `/status` | Show task/agent dashboard |
| `/task list` | List all tasks |
| `/task get <ID>` | Get task details |
| `/agent list` | List all agents |
| `/agent reputation <ID>` | Show agent reputation |
| `/config show` | Show current configuration |
| `/config init` | Initialize default config file |
| `/demo` | Run a full delegation lifecycle demo |
| `/help` | Show available commands |
| `/quit` | Exit the REPL |

Natural language input is routed through Claude (haiku) for intent classification and automatically dispatched to the appropriate command.

### Build from source

```bash
cargo build
cargo test
cargo clippy -- -D warnings
```

## Architecture

```
panopticon-ai (single crate)
  |
  +-- repl/            Interactive REPL (rustyline + colored)
  |     +-- slash        Slash command parser & dispatcher
  |     +-- router       Natural language intent router (Claude haiku)
  |     +-- session      Conversation context management
  |     +-- output       Colored output, prompts, welcome/help
  +-- cli/             Command handlers & application state
  +-- coordination/    Event-driven coordination loop
  +-- decomposition/   Task decomposition (Sequential / Parallel / Hybrid)
  +-- assignment/      Capability matching, RFP/bid, contract building
  +-- monitoring/      Async monitoring loop, SLO violation detection
  +-- verification/    4 verification strategies, dispute resolution
  +-- permissions/     Approval levels, privilege attenuation
  +-- security/        Threat detection, circuit breakers
  +-- optimizer/       Pareto front computation, multi-objective optimization
  +-- reputation/      EMA-based multi-dimensional scoring, trust levels
  +-- ledger/          Immutable audit ledger (in-memory / Merkle tree)
  +-- types/           Core domain types, state machines, error types
```

### Modules

| Module | Description |
|---|---|
| `types` | Task (11-dim characteristics, state machine), Agent, DelegationContract, DelegationChain, error types |
| `ledger` | `Ledger` trait + `InMemoryLedger` (default) + `MerkleLedger` (feature-gated) |
| `decomposition` | `DecompositionStrategy` trait + Sequential / Parallel / Hybrid implementations, DAG cycle detection |
| `reputation` | EMA-based scoring with adaptive learning rate, weighted composite (completion 0.4, quality 0.3, reliability 0.15, safety 0.1, behavioral 0.05) |
| `assignment` | `CapabilityMatcher`, RFP/Bid protocol, `ContractBuilder` |
| `optimizer` | Multi-objective evaluation, Pareto front computation, delegation overhead estimation |
| `monitoring` | Async monitoring loop (`tokio::select!`), checkpoint management, SLO violation detection |
| `coordination` | Event-driven coordinator mapping triggers (spec change, budget exceeded, agent unresponsive, ...) to responses (re-delegate, escalate, terminate, ...) |
| `verification` | 4 verifiers (Direct Inspection, Third-Party Audit, Cryptographic stub, Game-Theoretic), ed25519 credentials, dispute state machine |
| `security` | Sybil / Collusion / Behavioral threat detectors, circuit breaker with token revocation |
| `permissions` | Criticality x reversibility approval matrix (Standing / Contextual / JIT), privilege attenuation for re-delegation chains |
| `repl` | Interactive REPL loop, slash command dispatch, natural language intent routing via Claude, session context |
| `cli` | Command handlers and application state |

## Requirements

- Rust 1.85+ (Edition 2024)

## Usage

```bash
# Launch the REPL
panopticon

# Or via cargo
cargo run
```

All interaction happens inside the REPL. See the [Slash commands](#slash-commands) table above for available commands.

### Configuration

Configuration is stored in `~/.panopticon/config.toml` (or `$PANOPTICON_STATE_DIR/config.toml`).

| Key | Default | Description |
|---|---|---|
| `default_model` | `sonnet` | Claude model for plan/execute |
| `router_model` | `haiku` | Claude model for natural language intent routing |
| `max_context_messages` | `20` | Number of conversation messages retained in session |
| `max_turns` | `10` | Max turns per Claude agent execution |
| `permission_mode` | `bypassPermissions` | Permission mode for Claude CLI |
| `min_reputation_threshold` | `0.3` | Minimum reputation for agent assignment |
| `decomposition_strategy` | `hybrid` | Default decomposition strategy |

```bash
# Inside the REPL:
> /config init   # Create default config
> /config show   # View current config
```

## Task State Machine

```
Pending --> Decomposing --> AwaitingAssignment --> Negotiating --> Contracted
                                                                     |
                                                                     v
Failed <-- AwaitingVerification <-- InProgress
  |              |          |
  |  (retry)     v          v
  +--------> Completed   Disputed --> Completed
                                  --> Failed
```

## Feature Flags

| Flag | Default | Description |
|---|---|---|
| `memory-ledger` | Yes | In-memory ledger implementation |
| `merkle-ledger` | No | Merkle tree-backed ledger with cryptographic integrity proofs |

```bash
# Enable Merkle ledger
cargo build --features merkle-ledger
```

## License

MIT
