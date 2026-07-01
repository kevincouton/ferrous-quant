//! # Observer Pattern
//!
//! ## Financial Rationale
//!
//! Market data feeds are the lifeblood of trading systems. Multiple
//! consumers (strategies, risk engines, loggers, UI dashboards) need
//! to react to the same market events. The Observer pattern provides
//! a publish-subscribe mechanism decoupling producers from consumers.
//!
//! ## Rust Adaptation
//!
//! In Rust, we use `tokio::sync::broadcast` or `crossbeam::channel`
//! for multi-producer/multi-consumer event dissemination. This module
//! provides a trait-based abstraction that can be backed by either.
//!
//! For high-frequency scenarios, prefer lock-free channels. For
//! backtesting with single-threaded execution, a simple Vec<Box<dyn Observer>>
//! suffices.

use std::fmt::Debug;
use std::sync::Arc;

/// A generic event that can be observed.
pub trait Event: Debug + Clone + Send + Sync + 'static {}

// blanket impl for types that satisfy the bounds
impl<T> Event for T where T: Debug + Clone + Send + Sync + 'static {}

/// An observer receives events and reacts to them.
///
/// The `on_event` method is synchronous; for async reactions,
/// implementors should spawn tasks or use channels internally.
pub trait Observer<E: Event>: Debug + Send + Sync {
    /// Called when an event is published.
    fn on_event(&self, event: &E);
}

/// An observable subject maintains a list of observers and notifies them.
pub trait Observable<E: Event> {
    /// Attach an observer.
    fn attach(&mut self, observer: Arc<dyn Observer<E>>);

    /// Detach an observer.
    fn detach(&mut self, observer: &Arc<dyn Observer<E>>);

    /// Notify all attached observers.
    fn notify(&self, event: &E);
}

// ------------------------------------------------------------------
// In-Memory Observable (single-threaded or with Arc)
// ------------------------------------------------------------------

use parking_lot::RwLock;

/// Simple in-memory observable using `Arc` and `RwLock`.
///
/// Suitable for backtesting and low-latency scenarios where
/// observer registration is infrequent compared to events.
#[derive(Debug)]
pub struct InMemoryObservable<E: Event> {
    observers: RwLock<Vec<Arc<dyn Observer<E>>>>,
}

impl<E: Event> Default for InMemoryObservable<E> {
    fn default() -> Self {
        Self {
            observers: RwLock::new(Vec::new()),
        }
    }
}

impl<E: Event> InMemoryObservable<E> {
    /// Create a new observable.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<E: Event> Observable<E> for InMemoryObservable<E> {
    fn attach(&mut self, observer: Arc<dyn Observer<E>>) {
        self.observers.write().push(observer);
    }

    fn detach(&mut self, observer: &Arc<dyn Observer<E>>) {
        let ptr = Arc::as_ptr(observer);
        let mut obs = self.observers.write();
        obs.retain(|o| Arc::as_ptr(o) != ptr);
    }

    fn notify(&self, event: &E) {
        let observers = self.observers.read();
        for obs in observers.iter() {
            obs.on_event(event);
        }
    }
}

// ------------------------------------------------------------------
// Domain Example: Market Data Event
// ------------------------------------------------------------------

/// A market data tick event.
#[derive(Debug, Clone, PartialEq)]
pub struct TickEvent {
    /// Symbol (e.g., "AAPL").
    pub symbol: String,
    /// Best bid price.
    pub bid: f64,
    /// Best ask price.
    pub ask: f64,
    /// Last trade price.
    pub last: f64,
    /// Last trade size.
    pub size: u64,
    /// Timestamp (unix nanos).
    pub timestamp: u64,
}

/// A logger observer that prints ticks.
#[derive(Debug, Clone)]
pub struct TickLogger;

impl Observer<TickEvent> for TickLogger {
    fn on_event(&self, event: &TickEvent) {
        tracing::info!(
            symbol = %event.symbol,
            last = event.last,
            size = event.size,
            "tick received"
        );
    }
}

/// A risk observer that tracks bid-ask spread.
#[derive(Debug, Clone)]
pub struct SpreadMonitor {
    max_spread_bps: f64,
}

impl SpreadMonitor {
    /// Create a spread monitor with a maximum spread in basis points.
    pub fn new(max_spread_bps: f64) -> Self {
        Self { max_spread_bps }
    }
}

impl Observer<TickEvent> for SpreadMonitor {
    fn on_event(&self, event: &TickEvent) {
        let mid = (event.bid + event.ask) / 2.0;
        if mid == 0.0 {
            return;
        }
        let spread_bps = ((event.ask - event.bid) / mid) * 10_000.0;
        if spread_bps > self.max_spread_bps {
            tracing::warn!(
                symbol = %event.symbol,
                spread_bps = spread_bps,
                "spread exceeds threshold"
            );
        }
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Debug)]
    struct CounterObserver {
        count: AtomicU64,
    }

    impl Observer<TickEvent> for CounterObserver {
        fn on_event(&self, _event: &TickEvent) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn observable_notifies_all_observers() {
        let mut observable = InMemoryObservable::<TickEvent>::new();
        let obs1 = Arc::new(CounterObserver {
            count: AtomicU64::new(0),
        });
        let obs2 = Arc::new(CounterObserver {
            count: AtomicU64::new(0),
        });

        observable.attach(obs1.clone());
        observable.attach(obs2.clone());

        let event = TickEvent {
            symbol: "AAPL".into(),
            bid: 150.0,
            ask: 150.1,
            last: 150.05,
            size: 100,
            timestamp: 0,
        };

        observable.notify(&event);

        assert_eq!(obs1.count.load(Ordering::SeqCst), 1);
        assert_eq!(obs2.count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn detach_removes_observer() {
        let mut observable = InMemoryObservable::<TickEvent>::new();
        let obs = Arc::new(CounterObserver {
            count: AtomicU64::new(0),
        });

        let obs_dyn: Arc<dyn Observer<TickEvent>> = obs.clone();
        observable.attach(obs_dyn.clone());
        observable.detach(&obs_dyn);

        let event = TickEvent {
            symbol: "AAPL".into(),
            bid: 150.0,
            ask: 150.1,
            last: 150.05,
            size: 100,
            timestamp: 0,
        };

        observable.notify(&event);
        assert_eq!(obs.count.load(Ordering::SeqCst), 0);
    }
}
