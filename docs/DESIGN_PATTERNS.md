# Design Patterns in Ferrous Quant

This document catalogs the design patterns used across the Ferrous Quant workspace, their financial domain rationale, and their Rust implementation.

## Pattern Catalog

### 1. Builder Pattern

**Financial Use Case**: Order construction with validation. Financial orders have many optional parameters (limit price, stop price, TIF, OCO brackets, algo params) and invalid combinations must be rejected.

**Rust Implementation**: `quant-patterns::builder::OrderBuilder` — uses consuming builder methods and runtime validation.

```rust
let order = OrderBuilder::new()
    .symbol("AAPL")
    .quantity(100)
    .side(Side::Buy)
    .order_type(OrderType::Limit)
    .limit_price(150.00)
    .build()?;
```

### 2. Strategy Pattern

**Financial Use Case**: Trading algorithms (trend following, mean reversion, statistical arbitrage, ML-based). The backtesting engine must run diverse strategies uniformly.

**Rust Implementation**: `quant-patterns::strategy::Strategy` trait with `evaluate(&self, ctx: &MarketContext) -> Signal`.

```rust
pub trait Strategy: Debug + Send + Sync {
    fn evaluate(&self, ctx: &MarketContext) -> Signal;
}
```

### 3. Observer Pattern

**Financial Use Case**: Market data subscriptions. Multiple consumers (strategies, risk engines, loggers, dashboards) need to react to the same tick events.

**Rust Implementation**: `quant-patterns::observer::InMemoryObservable` using `Arc` + `RwLock` for thread-safe pub-sub.

### 4. Command Pattern

**Financial Use Case**: Order commands with complete audit trails. Every order placement, modification, and cancellation is a command that must be logged, replayed, and validated.

**Rust Implementation**: `quant-patterns::command::{Command, CommandBus}` — type-erased handlers with unique command IDs.

### 5. State Pattern (Typestate)

**Financial Use Case**: Order lifecycle (Pending → Submitted → Working → Filled/Cancelled). Invalid transitions (e.g., Filled → Cancelled) must be prevented.

**Rust Implementation**: Two variants:
- **Runtime**: `quant-patterns::state::StatefulOrder` with `StateMachine` trait
- **Compile-time**: `quant-patterns::state::TypestateOrder<State>` using `PhantomData` — zero-cost, invalid transitions are compile errors

### 6. Pipeline Pattern

**Financial Use Case**: Data transformation chains: Raw tick → Bar aggregation → Indicator computation → Signal generation → Risk check → Order command.

**Rust Implementation**: `quant-patterns::pipeline::{Pipeline, Processor}` — composable stages with error handling and internal buffering.

### 7. Actor Pattern

**Financial Use Case**: Concurrent execution engines. Market data, strategy evaluation, risk checks, and order execution run in parallel with isolated state.

**Rust Implementation**: `quant-patterns::actor::{Actor, ActorHandle}` using `tokio::sync::mpsc` and `async_trait` for message-driven concurrency.

### 8. Chain of Responsibility

**Financial Use Case**: Pre-trade risk validation. Orders must pass: symbol whitelist → max notional → credit check → compliance check before reaching the market.

**Rust Implementation**: `quant-patterns::chain_of_responsibility::{Chain, ChainHandler}` with `HandlerResult::{Approved, Rejected, Pass}`.

### 9. Repository Pattern

**Financial Use Case**: Data provider abstraction. Strategies should not depend on whether data comes from Yahoo Finance, IBKR, CSV, or a database.

**Rust Implementation**: `quant-patterns::repository::{Repository, DataQuery}` with `InMemoryRepository` for testing and `async_trait` for network sources.

### 10. Visitor Pattern

**Financial Use Case**: Portfolio analytics. Computing PnL, delta, gamma, VaR, and sector exposure over a heterogeneous portfolio (equities, options, futures, composites).

**Rust Implementation**: `quant-patterns::visitor::{Visitor, Visitable}` with double dispatch via `accept()` and `visit_*()` methods.

### 11. Newtype Pattern

**Financial Use Case**: Type-safe prices and quantities. Preventing the billion-dollar mistake of adding a price to a quantity, or confusing USD and JPY amounts.

**Rust Implementation**: `quant-core::price::{Price, Quantity}` — wraps `rust_decimal::Decimal` with domain-specific operations and validation.

### 12. Typestate Pattern

**Financial Use Case**: Compile-time order state validation. A `Pending` order can be submitted or cancelled, but a `Filled` order cannot be cancelled.

**Rust Implementation**: `quant-core::order::Order<S: OrderState>` using generics and `PhantomData`. Zero-cost at runtime.

## Performance Considerations

| Pattern | Runtime Cost | Notes |
|---------|-------------|-------|
| Builder | O(1) per field | Consuming methods prevent reuse bugs |
| Strategy (dyn) | Vtable dispatch | Use `impl Strategy` for hot paths |
| Observer | O(n) observers | Lock-free channels for HFT |
| Command | Heap allocation | Use `enum` commands for zero-cost |
| Typestate | Zero | Erased at compile time |
| Pipeline | Stage overhead | Batch processing reduces per-item cost |
| Actor | Channel send | Unbounded channels for backpressure awareness |
| Chain | O(handlers) | Early exit on rejection minimizes cost |
| Repository | Network / IO | Cache hot data in `quant-data` |
| Visitor | Double dispatch | Prefer `enum` dispatch for closed hierarchies |
| Newtype | Zero | `Price` and `Quantity` are ZST wrappers over `Decimal` |

## When to Choose Which

- **Hot path (HFT)**: Prefer monomorphized generics, `enum` dispatch, and lock-free data structures. Avoid `Box<dyn>` and `Arc`.
- **Backtesting engine**: Use `dyn Strategy` and `dyn Repository` for flexibility. Vtable overhead is negligible compared to data processing.
- **Order management**: Use typestate for compile-time safety and runtime state machine for persistence/serialization.
- **Risk checks**: Chain of responsibility with early exit. Most orders are rejected by the first handler.
- **Portfolio analytics**: Visitor pattern for extensibility. New analytics can be added without modifying position types.
