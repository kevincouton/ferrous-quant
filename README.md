# Ferrous Quant

> A comprehensive, educational, production-grade quantitative finance platform written in Rust.
>
> **Mission**: Rationalize quantitative finance domain knowledge through idiomatic Rust, explicit design patterns, and rigorous testing.

[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-unit%20%7C%20integration%20%7C%20e2e%20%7C%20perf-green)](docs/TESTING.md)

---

## Table of Contents

- [Philosophy](#philosophy)
- [Architecture](#architecture)
- [Workspace Crates](#workspace-crates)
- [Design Patterns](#design-patterns)
- [Quick Start](#quick-start)
- [Testing Strategy](#testing-strategy)
- [Docker / Podman](#docker--podman)
- [Data Providers](#data-providers)
- [Educational Resources](#educational-resources)
- [References](#references)
- [Contributing](#contributing)

---

## Philosophy

This project is built on three pillars:

1. **Education First**: Every module explains the *why* behind the implementation. Design patterns are explicit, documented, and justified.
2. **Rust Idioms**: Zero-cost abstractions, fearless concurrency, and type-safe domain modeling. If a pattern exists in another language but not Rust, we reimplement it as a first-class crate.
3. **Production Grade**: Comprehensive testing (unit, integration, e2e, performance), Docker containerization, and observability from day one.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         ferrous-quant CLI / API                      │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌───────────┐  │
│  │  Strategies │  │  Backtest   │  │  Execution  │  │   Risk    │  │
│  │   Engine    │  │   Engine    │  │   Engine    │  │  Engine   │  │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └─────┬─────┘  │
│         │                │                │               │        │
│  ┌──────┴────────────────┴────────────────┴───────────────┘        │
│  │                    quant-core (Domain Primitives)                │
│  │  Symbol, Price, Quantity, OHLCV, TimeSeries, Currency, Tick   │
│  └────────────────────────────────────────────────────────────────┘  │
│         │                │                │               │        │
│  ┌──────┴────────────────┴────────────────┴───────────────┘        │
│  │                    quant-data (Data Providers)                  │
│  │  Yahoo Finance │ IBKR TWS │ FRED │ CSV │ Parquet │ In-Memory   │
│  └────────────────────────────────────────────────────────────────┘  │
│         │                                                          │
│  ┌──────┴────────────────────────────────────────────────────────┐│
│  │                    quant-indicators (Technical Analysis)       ││
│  │  SMA, EMA, RSI, MACD, Bollinger Bands, ATR, VWAP, ...         ││
│  └────────────────────────────────────────────────────────────────┘│
│         │                                                          │
│  ┌──────┴────────────────────────────────────────────────────────┐│
│  │                    quant-patterns (Design Patterns)           ││
│  │  Builder, Strategy, Observer, Command, State, Pipeline,    ││
│  │  Actor, Chain of Responsibility, Visitor, Repository           ││
│  └────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

---

## Workspace Crates

| Crate | Purpose | Patterns Used |
|-------|---------|---------------|
| `quant-patterns` | Reusable design patterns for financial systems | Builder, Strategy, Observer, Command, State, Pipeline, Actor, Chain of Responsibility, Repository |
| `quant-core` | Domain primitives and type system | Newtype, Typestate, PhantomData, Zero-cost abstractions |
| `quant-indicators` | Technical analysis indicators | Iterator, Strategy, Builder |
| `quant-data` | Data ingestion abstractions and providers | Repository, Adapter, Factory |
| `quant-ibkr` | Interactive Brokers TWS API client | Builder, Command, Observer, State Machine |
| `quant-backtest` | Event-driven backtesting engine | Event Sourcing, Strategy, Observer, Pipeline |
| `quant-risk` | Risk metrics and portfolio analytics | Visitor, Strategy, Builder |
| `quant-execution` | Order execution and paper trading | Command, State, Chain of Responsibility |
| `quant-cli` | Command-line interface | Builder, Command |
| `quant-book` | Educational documentation and runnable examples | - |

---

## Design Patterns

We adopt and adapt classical design patterns for Rust's ownership and trait system. Each pattern is:

1. **Documented** with financial domain rationale
2. **Implemented** as a reusable generic crate component
3. **Tested** with unit, integration, and property-based tests
4. **Benchmarked** where performance matters

See [`docs/DESIGN_PATTERNS.md`](docs/DESIGN_PATTERNS.md) for the full catalog.

### Pattern Catalog

| Pattern | Financial Use Case | Rust Implementation |
|---------|--------------------|---------------------|
| **Builder** | Order construction, strategy configuration | `OrderBuilder`, `StrategyBuilder` |
| **Strategy** | Trading algorithms, risk models | ` trait TradingStrategy` |
| **Observer** | Market data subscriptions, event notifications | ` trait MarketDataObserver` + channels |
| **Command** | Order commands, audit trail | ` trait OrderCommand` + `CommandBus` |
| **State** | Order lifecycle (Pending → Filled → Cancelled) | ` enum OrderState` + state transitions |
| **Pipeline** | Data ETL, indicator computation chains | ` struct Pipeline` + ` trait Processor` |
| **Actor** | Concurrent execution engines, market data handlers | ` trait Actor` + `tokio::sync::mpsc` |
| **Chain of Responsibility** | Risk checks, order validation | ` trait RiskCheck` + `RiskCheckChain` |
| **Repository** | Data access abstraction | ` trait DataRepository` |
| **Visitor** | Portfolio analytics, risk aggregation | ` trait PortfolioVisitor` |
| **Newtype** | Type-safe prices, quantities, currencies | ` struct Price(f64); struct Quantity(f64);` |
| **Typestate** | Compile-time state validation | ` struct Order<State: OrderStateTrait>` |

---

## Quick Start

### Prerequisites

- Rust 1.85+ (`rustup update stable`)
- Docker or Podman
- `just` (task runner) — `cargo install just`

### Clone & Build

```bash
git clone https://github.com/kevincouton/ferrous-quant.git
cd ferrous-quant
cargo build --workspace
```

### Run with Podman

```bash
# Start the full stack
just podman-up

# Or individually
podman-compose up -d

# Run the CLI
podman run --rm -it ferrous-quant-cli:latest --help
```

### Run Tests

```bash
# All tests
cargo test --workspace

# With coverage
cargo llvm-cov --workspace --html

# Benchmarks
cargo bench --workspace

# E2E tests (requires IB Gateway paper account)
cargo test --workspace --features e2e
```

---

## Testing Strategy

| Level | Scope | Tools | Location |
|-------|-------|-------|----------|
| **Unit** | Individual functions, types | `#[test]` + `proptest` | `src/` inline |
| **Integration** | Module interactions, APIs | `tests/` directories | `tests/integration/` |
| **E2E** | Full workflows, broker APIs | `tests/e2e/` + Docker | `tests/e2e/` |
| **Performance** | Latency, throughput | `criterion.rs` | `benches/` |
| **Property** | Invariants, correctness | `proptest` | Mixed |
| **Fuzz** | Edge cases, panics | `cargo-fuzz` | `fuzz/` |

See [`docs/TESTING.md`](docs/TESTING.md) for detailed testing philosophy and examples.

---

## Docker / Podman

All services run in containers using **Podman** locally:

```bash
# Development stack
podman-compose -f compose.dev.yml up -d

# Services:
# - IB Gateway (paper trading)
# - PostgreSQL (audit trail)
# - ClickHouse (analytics)
# - Grafana (monitoring)
# - Redpanda (event streaming)
```

See [`deploy/`](deploy/) for Kubernetes manifests and [`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md).

---

## Data Providers

| Provider | Crate | Status | Notes |
|----------|-------|--------|-------|
| **Yahoo Finance** | `quant-data` | ✅ | Free, historical + real-time |
| **Interactive Brokers** | `quant-ibkr` | ✅ | TWS API, live + paper trading |
| **FRED** | `quant-data` | 🔄 | Macroeconomic data |
| **CoinMetrics** | `quant-data` | 🔄 | Crypto on-chain data |
| **CSV / Parquet** | `quant-data` | ✅ | Local file ingestion |
| **In-Memory** | `quant-data` | ✅ | Test fixtures, synthetic data |

---

## Educational Resources

The `quant-book/` crate contains runnable examples and deep-dives:

- [`quant-book/src/ch01_design_patterns.rs`](quant-book/src/ch01_design_patterns.rs) — Design Patterns in Financial Systems
- [`quant-book/src/ch02_market_data.rs`](quant-book/src/ch02_market_data.rs) — Modeling Market Data
- [`quant-book/src/ch03_technical_indicators.rs`](quant-book/src/ch03_technical_indicators.rs) — Technical Indicator Implementations
- [`quant-book/src/ch04_backtesting.rs`](quant-book/src/ch04_backtesting.rs) — Event-Driven Backtesting
- [`quant-book/src/ch05_risk_management.rs`](quant-book/src/ch05_risk_management.rs) — Risk Metrics and Portfolio Theory
- [`quant-book/src/ch06_ibkr_integration.rs`](quant-book/src/ch06_ibkr_integration.rs) — Connecting to Interactive Brokers

Each chapter is a standalone binary you can run:

```bash
cargo run --bin ch01_design_patterns
```

---

## References

This project draws inspiration from the quant community:

- [awesome-quant](https://github.com/wilsonfreitas/awesome-quant) by Wilson Freitas — comprehensive library catalog
- [awesome-quant-ai](https://github.com/leoncuhk/awesome-quant-ai) by leoncuhk — AI/ML in quant finance
- [NautilusTrader](https://github.com/nautechsystems/nautilus_trader) — high-performance event-driven trading platform
- [RustQuant](https://github.com/avhz/RustQuant) — quantitative finance library in Rust
- [Barter](https://github.com/barter-rs/barter-rs) — Rust trading framework

Key texts informing the architecture:

- *Advances in Financial Machine Learning* by Marcos Lopez de Prado
- *Systematic Trading* by Robert Carver
- *Algorithmic Trading* by Ernest Chan
- *Building Reliable Trading Systems* by Keith Fitschen

---

## Contributing

Contributions are welcome. Please read [`CONTRIBUTING.md`](CONTRIBUTING.md) for guidelines.

All code must pass:
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test --workspace`
- `cargo doc --workspace --no-deps`

---

## License

Dual-licensed under MIT or Apache-2.0.

---

<div align="center">

**Built with 🦀 Rust for quantitative finance**

</div>
