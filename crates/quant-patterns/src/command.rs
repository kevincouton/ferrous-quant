//! # Command Pattern
//!
//! ## Financial Rationale
//!
//! Every order placed in a trading system is a *command*: a request to
//! perform an action with a complete audit trail. The Command pattern
//! encapsulates orders as objects, enabling:
//! - Queuing and deferred execution
//! - Undo/redo (order cancellation)
//! - Logging and audit trails (regulatory requirement)
//! - Transactional batching (e.g., basket orders)
//!
//! ## Rust Adaptation
//!
//! Commands are pure data structures that implement a `Command` trait.
//! A `CommandBus` routes them to handlers. We use `tokio::sync::mpsc`
//! for async execution and `tracing` for structured logging.

use std::fmt::Debug;

/// A unique command identifier for audit trails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandId(pub u64);

/// A command represents an intent to perform an action.
///
/// Commands are immutable, serializable, and replayable.
/// They form the basis of event sourcing and audit logs.
pub trait Command: Debug + Send + Sync + 'static {
    /// Unique identifier for this command instance.
    fn id(&self) -> CommandId;

    /// Human-readable description for logging.
    fn description(&self) -> String;
}

/// A command handler executes a specific command type.
pub trait CommandHandler<C: Command>: Send + Sync {
    /// The result of executing the command.
    type Result: Debug + Send;

    /// The error type if execution fails.
    type Error: std::error::Error + Send + Sync;

    /// Execute the command. May be async in real implementations.
    fn handle(&self, command: &C) -> Result<Self::Result, Self::Error>;
}

// ------------------------------------------------------------------
// Domain Example: Trading Commands
// ------------------------------------------------------------------

use std::sync::atomic::{AtomicU64, Ordering};

static CMD_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_cmd_id() -> CommandId {
    CommandId(CMD_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
}

/// Place a new order.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaceOrderCommand {
    id: CommandId,
    pub symbol: String,
    pub quantity: u64,
    pub side: super::builder::Side,
    pub order_type: super::builder::OrderType,
    pub limit_price: Option<f64>,
}

impl PlaceOrderCommand {
    /// Create a new place-order command.
    pub fn new(
        symbol: impl Into<String>,
        quantity: u64,
        side: super::builder::Side,
        order_type: super::builder::OrderType,
        limit_price: Option<f64>,
    ) -> Self {
        Self {
            id: next_cmd_id(),
            symbol: symbol.into(),
            quantity,
            side,
            order_type,
            limit_price,
        }
    }
}

impl Command for PlaceOrderCommand {
    fn id(&self) -> CommandId {
        self.id
    }

    fn description(&self) -> String {
        format!(
            "PlaceOrder: {:?} {} {} {:?}",
            self.side, self.quantity, self.symbol, self.order_type
        )
    }
}

/// Cancel an existing order.
#[derive(Debug, Clone, PartialEq)]
pub struct CancelOrderCommand {
    id: CommandId,
    pub order_id: String,
}

impl CancelOrderCommand {
    /// Create a new cancel-order command.
    pub fn new(order_id: impl Into<String>) -> Self {
        Self {
            id: next_cmd_id(),
            order_id: order_id.into(),
        }
    }
}

impl Command for CancelOrderCommand {
    fn id(&self) -> CommandId {
        self.id
    }

    fn description(&self) -> String {
        format!("CancelOrder: {}", self.order_id)
    }
}

/// Modify an existing order (e.g., change limit price).
#[derive(Debug, Clone, PartialEq)]
pub struct ModifyOrderCommand {
    id: CommandId,
    pub order_id: String,
    pub new_limit_price: Option<f64>,
    pub new_quantity: Option<u64>,
}

impl ModifyOrderCommand {
    /// Create a new modify-order command.
    pub fn new(order_id: impl Into<String>) -> Self {
        Self {
            id: next_cmd_id(),
            order_id: order_id.into(),
            new_limit_price: None,
            new_quantity: None,
        }
    }

    /// Set the new limit price.
    pub fn limit_price(mut self, price: f64) -> Self {
        self.new_limit_price = Some(price);
        self
    }

    /// Set the new quantity.
    pub fn quantity(mut self, qty: u64) -> Self {
        self.new_quantity = Some(qty);
        self
    }
}

impl Command for ModifyOrderCommand {
    fn id(&self) -> CommandId {
        self.id
    }

    fn description(&self) -> String {
        format!("ModifyOrder: {}", self.order_id)
    }
}

// ------------------------------------------------------------------
// Command Bus (Router)
// ------------------------------------------------------------------

use std::collections::HashMap;

/// A type-erased command handler for dynamic dispatch.
type ErasedHandler = Box<
    dyn Fn(
            &dyn std::any::Any,
        )
            -> Result<Box<dyn std::any::Any + Send>, Box<dyn std::error::Error + Send + Sync>>
        + Send
        + Sync,
>;

/// Routes commands to their registered handlers.
#[derive(Default)]
pub struct CommandBus {
    handlers: HashMap<std::any::TypeId, ErasedHandler>,
}

impl CommandBus {
    /// Create an empty command bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler for a specific command type.
    pub fn register<C, H>(&mut self, handler: H)
    where
        C: Command + 'static,
        H: CommandHandler<C> + 'static,
    {
        let boxed = Box::new(move |cmd: &dyn std::any::Any| {
            let concrete = cmd.downcast_ref::<C>().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "type mismatch")
            })?;
            let result = handler.handle(concrete)?;
            Ok(Box::new(result) as Box<dyn std::any::Any + Send>)
        });
        self.handlers.insert(std::any::TypeId::of::<C>(), boxed);
    }

    /// Execute a command, returning the handler's result.
    pub fn execute<C>(
        &self,
        command: &C,
    ) -> Result<Box<dyn std::any::Any + Send>, Box<dyn std::error::Error + Send + Sync>>
    where
        C: Command + 'static,
    {
        let handler = self
            .handlers
            .get(&std::any::TypeId::of::<C>())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "no handler registered")
            })?;
        handler(command as &dyn std::any::Any)
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct PlaceOrderHandler;

    impl CommandHandler<PlaceOrderCommand> for PlaceOrderHandler {
        type Result = String;
        type Error = std::io::Error;

        fn handle(&self, cmd: &PlaceOrderCommand) -> Result<Self::Result, Self::Error> {
            Ok(format!("placed: {}", cmd.symbol))
        }
    }

    #[test]
    fn command_bus_routes_to_handler() {
        let mut bus = CommandBus::new();
        bus.register(PlaceOrderHandler);

        let cmd = PlaceOrderCommand::new(
            "AAPL",
            100,
            super::super::builder::Side::Buy,
            super::super::builder::OrderType::Market,
            None,
        );
        let result = bus.execute(&cmd).unwrap();
        let s = result.downcast_ref::<String>().unwrap();
        assert_eq!(s, "placed: AAPL");
    }

    #[test]
    fn command_id_is_unique() {
        let a = next_cmd_id();
        let b = next_cmd_id();
        assert_ne!(a, b);
    }
}
