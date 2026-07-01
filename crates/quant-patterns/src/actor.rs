//! # Actor Pattern
//!
//! ## Financial Rationale
//!
//! Trading systems are inherently concurrent: market data arrives on one
//! thread, strategy evaluation runs on another, and order execution happens
//! on a third. The Actor pattern isolates state and message handling,
//! preventing data races and simplifying reasoning about concurrent systems.
//!
//! ## Rust Adaptation
//!
//! In Rust, we use `tokio::sync::mpsc` for async actors and `crossbeam::channel`
//! for sync actors. This module provides a trait abstraction over both.

use std::fmt::Debug;
use tokio::sync::mpsc;

/// A message that can be sent to an actor.
pub trait Message: Debug + Send + 'static {}

impl<T> Message for T where T: Debug + Send + 'static {}

/// An actor receives messages and processes them sequentially.
///
/// The actor's state is isolated; no external code can access it directly.
/// All interaction is through message passing.
#[async_trait::async_trait]
pub trait Actor: Debug + Send + 'static {
    /// The type of messages this actor handles.
    type Msg: Message;

    /// Handle a single message. Called sequentially for each message.
    async fn handle(&mut self, msg: Self::Msg);
}

/// A handle to an actor that can send messages.
///
/// The actor itself runs in a dedicated task. The handle is cloneable
/// and can be shared among multiple producers.
#[derive(Debug)]
pub struct ActorHandle<M: Message> {
    sender: mpsc::UnboundedSender<M>,
}

impl<M: Message> Clone for ActorHandle<M> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<M: Message> ActorHandle<M> {
    /// Send a message to the actor. Fails if the actor has stopped.
    pub fn send(&self, msg: M) -> Result<(), ActorError> {
        self.sender.send(msg).map_err(|_| ActorError::ActorStopped)
    }

    /// Try to send without blocking. Same semantics as `send` for unbounded.
    pub fn try_send(&self, msg: M) -> Result<(), ActorError> {
        self.send(msg)
    }
}

/// Errors from actor operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ActorError {
    /// The actor task has terminated.
    ActorStopped,
}

impl std::fmt::Display for ActorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorError::ActorStopped => write!(f, "actor has stopped"),
        }
    }
}

impl std::error::Error for ActorError {}

/// Spawn an actor into a new Tokio task and return its handle.
///
/// The actor runs a loop that receives messages and dispatches them
/// to `Actor::handle`. When all handles are dropped, the channel closes
/// and the actor task terminates.
pub fn spawn_actor<A>(mut actor: A) -> ActorHandle<A::Msg>
where
    A: Actor,
{
    let (tx, mut rx) = mpsc::unbounded_channel::<A::Msg>();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            actor.handle(msg).await;
        }
        tracing::info!(actor = ?std::any::type_name::<A>(), "actor stopped");
    });

    ActorHandle { sender: tx }
}

// ------------------------------------------------------------------
// Domain Example: Risk Check Actor
// ------------------------------------------------------------------

/// Message sent to the risk actor.
#[derive(Debug, Clone, PartialEq)]
pub enum RiskMessage {
    /// Check if an order is within risk limits.
    CheckOrder {
        symbol: String,
        quantity: u64,
        notional: f64,
    },
    /// Query current exposure for a symbol.
    QueryExposure { symbol: String },
    /// Reset all exposures (e.g., end of day).
    Reset,
}

/// Risk actor that tracks per-symbol exposure and enforces limits.
#[derive(Debug)]
pub struct RiskActor {
    max_notional_per_symbol: f64,
    exposures: std::collections::HashMap<String, f64>,
}

impl RiskActor {
    /// Create a risk actor with a notional limit per symbol.
    pub fn new(max_notional_per_symbol: f64) -> Self {
        Self {
            max_notional_per_symbol,
            exposures: std::collections::HashMap::new(),
        }
    }
}

#[async_trait::async_trait]
impl Actor for RiskActor {
    type Msg = RiskMessage;

    async fn handle(&mut self, msg: RiskMessage) {
        match msg {
            RiskMessage::CheckOrder {
                symbol,
                quantity: _,
                notional,
            } => {
                let current = self.exposures.get(&symbol).copied().unwrap_or(0.0);
                let would_be = current + notional.abs();
                if would_be > self.max_notional_per_symbol {
                    tracing::warn!(
                        symbol = %symbol,
                        current = current,
                        would_be = would_be,
                        limit = self.max_notional_per_symbol,
                        "risk limit exceeded"
                    );
                } else {
                    self.exposures.insert(symbol.clone(), would_be);
                    tracing::info!(symbol = %symbol, notional = notional, "order approved by risk");
                }
            }
            RiskMessage::QueryExposure { symbol } => {
                let exp = self.exposures.get(&symbol).copied().unwrap_or(0.0);
                tracing::info!(symbol = %symbol, exposure = exp, "risk exposure queried");
            }
            RiskMessage::Reset => {
                self.exposures.clear();
                tracing::info!("risk exposures reset");
            }
        }
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestMsg(u64);

    #[derive(Debug)]
    struct TestActor {
        sum: u64,
    }

    #[async_trait::async_trait]
    impl Actor for TestActor {
        type Msg = TestMsg;

        async fn handle(&mut self, msg: TestMsg) {
            self.sum += msg.0;
        }
    }

    #[tokio::test]
    async fn actor_handle_is_cloneable() {
        let handle = spawn_actor(TestActor { sum: 0 });
        let _clone = handle.clone();
    }

    #[tokio::test]
    async fn actor_processes_messages() {
        let handle = spawn_actor(TestActor { sum: 0 });
        handle.send(TestMsg(10)).unwrap();
        handle.send(TestMsg(20)).unwrap();
        // Give the actor a moment to process
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}
