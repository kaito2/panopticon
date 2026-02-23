<p align="center">
  <img src="docs/images/hero.jpg" alt="Panopticon" width="800">
</p>

# Panopticon

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

## Architecture

```
panopticon-ai (single crate)
  |
  +-- cli/             CLI interface (clap)
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
| `cli` | CLI interface for tasks, agents, reputation, and state transitions |

## Requirements

- Rust 1.85+ (Edition 2024)

## Quick Start

```bash
# Build
cargo build

# Run tests
cargo test

# Run clippy
cargo clippy -- -D warnings

# Run full delegation lifecycle demo
cargo run -- demo
```

## CLI Usage

```bash
# Binary name
panopticon <command>

# Or via cargo
cargo run -- <command>
```

### Commands

| Command | Description |
|---|---|
| `demo` | Run a full delegation lifecycle demo |
| `task create` | Create a task with characteristics |
| `task list` | List all tasks |
| `task get <ID>` | Get task details |
| `task transition <ID> <EVENT>` | Apply a state transition event |
| `task decompose <ID> --strategy <NAME>` | Decompose a task (sequential, parallel, hybrid) |
| `agent register <NAME>` | Register an agent |
| `agent list` | List all agents |
| `agent get <ID>` | Get agent details |
| `agent reputation <ID>` | Get reputation score and trust level |

### Examples

```bash
# Run the demo (recommended first step)
panopticon demo

# Create a task
panopticon task create "Analyze data" "Process market data" \
  --complexity 0.7 --criticality 0.6

# Register an agent with capabilities
panopticon agent register analyst --capabilities "data_analysis,market_research"

# Decompose a task
panopticon task decompose <TASK_ID> --strategy hybrid
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
