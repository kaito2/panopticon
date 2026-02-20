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
panopticon-server         HTTP API (Axum)
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
| `panopticon-server` | Axum HTTP API for tasks, agents, reputation, and state transitions |

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
cargo run -p panopticon-server --example basic_delegation

# Start the API server
cargo run -p panopticon-server
```

## API Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/health` | Health check |
| `POST` | `/api/v1/tasks` | Create a task |
| `GET` | `/api/v1/tasks` | List all tasks |
| `GET` | `/api/v1/tasks/{id}` | Get a task |
| `POST` | `/api/v1/tasks/{id}/transition` | Apply a state transition event |
| `POST` | `/api/v1/agents` | Register an agent |
| `GET` | `/api/v1/agents` | List all agents |
| `GET` | `/api/v1/agents/{id}` | Get an agent |
| `GET` | `/api/v1/agents/{id}/reputation` | Get reputation score and trust level |

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
