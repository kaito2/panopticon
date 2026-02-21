# Panopticon

Intelligent AI Delegation Framework -- an adaptive framework for dynamic and safe task delegation between AI agents.

Based on [Intelligent AI Delegation](https://arxiv.org/abs/2602.11865) (Tomasev, Franklin, Osindero — Google DeepMind, 2026).

## Overview

Panopticon implements the five pillars of the paper:

1. **Dynamic Evaluation** -- 11-dimensional task characterization (complexity, criticality, uncertainty, verifiability, reversibility, etc.) drives all delegation decisions.
2. **Adaptive Execution** -- Event-driven coordination loop with automatic re-delegation, re-decomposition, and escalation on failures.
3. **Structural Transparency** -- Immutable audit ledger (in-memory or Merkle tree-backed) records every action across the delegation lifecycle.
4. **Scalable Market Coordination** -- RFP/bid protocol with Pareto-optimal multi-objective selection (cost, quality, latency, uncertainty, privacy risk).
5. **System Resilience** -- Circuit breakers, threat detection (Sybil, collusion, behavioral anomalies), and privilege attenuation on re-delegation.

## Architecture

```
panopticon-cli            CLI interface (clap)
  |
  +-- panopticon-coordination    Event-driven coordination loop
  |     +-- panopticon-decomposition   Task decomposition (Sequential / Parallel / Hybrid)
  |     +-- panopticon-assignment      Capability matching, RFP/bid, contract building
  |     +-- panopticon-monitoring      Async monitoring loop, SLO violation detection
  |     +-- panopticon-verification    4 verification strategies, dispute resolution
  |     +-- panopticon-permissions     Approval levels, privilege attenuation
  |     +-- panopticon-security        Threat detection, circuit breakers
  |
  +-- panopticon-optimizer       Pareto front computation, multi-objective optimization
  +-- panopticon-reputation      EMA-based multi-dimensional scoring, trust levels
  +-- panopticon-ledger          Immutable audit ledger (in-memory / Merkle tree)
  +-- panopticon-types           Core domain types, state machines, error types
```

### Crates

| Crate | Description |
|---|---|
| `panopticon-types` | Task (11-dim characteristics, state machine), Agent, DelegationContract, DelegationChain, error types |
| `panopticon-ledger` | `Ledger` trait + `InMemoryLedger` (default) + `MerkleLedger` (feature-gated) |
| `panopticon-decomposition` | `DecompositionStrategy` trait + Sequential / Parallel / Hybrid implementations, DAG cycle detection |
| `panopticon-reputation` | EMA-based scoring with adaptive learning rate, weighted composite (completion 0.4, quality 0.3, reliability 0.15, safety 0.1, behavioral 0.05) |
| `panopticon-assignment` | `CapabilityMatcher`, RFP/Bid protocol, `ContractBuilder` |
| `panopticon-optimizer` | Multi-objective evaluation, Pareto front computation, delegation overhead estimation |
| `panopticon-monitoring` | Async monitoring loop (`tokio::select!`), checkpoint management, SLO violation detection |
| `panopticon-coordination` | Event-driven coordinator mapping triggers (spec change, budget exceeded, agent unresponsive, ...) to responses (re-delegate, escalate, terminate, ...) |
| `panopticon-verification` | 4 verifiers (Direct Inspection, Third-Party Audit, Cryptographic stub, Game-Theoretic), ed25519 credentials, dispute state machine |
| `panopticon-security` | Sybil / Collusion / Behavioral threat detectors, circuit breaker with token revocation |
| `panopticon-permissions` | Criticality x reversibility approval matrix (Standing / Contextual / JIT), privilege attenuation for re-delegation chains |
| `panopticon-cli` | CLI interface for tasks, agents, reputation, and state transitions |

## Requirements

- Rust 1.85+ (Edition 2024)

## Quick Start

```bash
# Build
cargo build --workspace

# Run tests (142 tests)
cargo test --workspace

# Run clippy
cargo clippy --workspace -- -D warnings

# Run the example
cargo run -p panopticon-cli --example basic_delegation

# Run full delegation lifecycle demo
cargo run -p panopticon-cli -- demo
```

## CLI Usage

```bash
# Binary name
panopticon <command>

# Or via cargo
cargo run -p panopticon-cli -- <command>
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
cargo build -p panopticon-ledger --features merkle-ledger
```

## License

MIT
