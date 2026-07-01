//! # Builder Pattern
//!
//! ## Financial Rationale
//!
//! Financial orders are complex objects with many optional parameters:
//! limit price, stop price, time-in-force, OCO brackets, algo parameters,
//! and route directives. The Builder pattern lets us construct these
//! incrementally with compile-time safety and fluent ergonomics.
//!
//! ## Rust Adaptation
//!
//! In Rust, we leverage move semantics and the type system to ensure:
//! - Required fields are set (via typestate or runtime validation)
//! - Invalid combinations are rejected at compile time where possible
//! - The builder is consumed on build, preventing reuse bugs
//!
//! ## Example
//!
//! ```rust
//! use quant_patterns::builder::{Builder, OrderBuilder, Side, OrderType};
//!
//! let order = OrderBuilder::new()
//!     .symbol("AAPL")
//!     .quantity(100)
//!     .side(Side::Buy)
//!     .order_type(OrderType::Limit)
//!     .limit_price(150.00)
//!     .build()
//!     .expect("valid order");
//! ```

use std::fmt;

/// Core builder trait. Implementors define a target type `T` and
/// a `build()` method that may fail if required fields are missing.
pub trait Builder<T> {
    /// The error type returned when build fails.
    type Error: fmt::Display;

    /// Consume the builder and attempt to construct `T`.
    fn build(self) -> Result<T, Self::Error>;
}

/// Marker trait for types that can be constructed via a builder.
pub trait Buildable: Sized {
    /// The builder type for this constructible.
    type Builder: Builder<Self>;

    /// Return a fresh builder for this type.
    fn builder() -> Self::Builder;
}

// ------------------------------------------------------------------
// Domain Example: Order Builder
// ------------------------------------------------------------------

/// Trading side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Buy (long) position.
    Buy,
    /// Sell (short) position.
    Sell,
}

/// Order type discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    /// Market order — execute immediately at best available price.
    Market,
    /// Limit order — execute at specified price or better.
    Limit,
    /// Stop order — trigger at specified stop price, then execute as market.
    Stop,
    /// Stop-limit order — trigger at stop, then submit as limit.
    StopLimit,
}

/// Time-in-force instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInForce {
    /// Good till cancelled.
    Gtc,
    /// Immediate or cancel.
    Ioc,
    /// Fill or kill.
    Fok,
    /// Day order.
    Day,
}

/// A validated trading order.
#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    symbol: String,
    quantity: u64,
    side: Side,
    order_type: OrderType,
    limit_price: Option<f64>,
    stop_price: Option<f64>,
    time_in_force: TimeInForce,
}

impl Order {
    /// Symbol being traded (e.g., "AAPL").
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Number of shares/contracts.
    pub fn quantity(&self) -> u64 {
        self.quantity
    }

    /// Buy or Sell.
    pub fn side(&self) -> Side {
        self.side
    }

    /// Order type.
    pub fn order_type(&self) -> OrderType {
        self.order_type
    }

    /// Limit price, if applicable.
    pub fn limit_price(&self) -> Option<f64> {
        self.limit_price
    }

    /// Stop price, if applicable.
    pub fn stop_price(&self) -> Option<f64> {
        self.stop_price
    }

    /// Time-in-force instruction.
    pub fn time_in_force(&self) -> TimeInForce {
        self.time_in_force
    }
}

/// Errors that can occur during order construction.
#[derive(Debug, Clone, PartialEq)]
pub enum OrderBuildError {
    /// Symbol is empty or missing.
    MissingSymbol,
    /// Quantity must be > 0.
    InvalidQuantity,
    /// Limit price required for Limit orders.
    MissingLimitPrice,
    /// Stop price required for Stop / StopLimit orders.
    MissingStopPrice,
    /// Limit price should not be present for Market orders.
    UnexpectedLimitPrice,
    /// Stop price should not be present for Market / Limit orders.
    UnexpectedStopPrice,
}

impl fmt::Display for OrderBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderBuildError::MissingSymbol => write!(f, "symbol is required"),
            OrderBuildError::InvalidQuantity => write!(f, "quantity must be > 0"),
            OrderBuildError::MissingLimitPrice => {
                write!(f, "limit price is required for Limit orders")
            }
            OrderBuildError::MissingStopPrice => {
                write!(f, "stop price is required for Stop / StopLimit orders")
            }
            OrderBuildError::UnexpectedLimitPrice => {
                write!(f, "limit price should not be set for Market orders")
            }
            OrderBuildError::UnexpectedStopPrice => {
                write!(f, "stop price should not be set for Market / Limit orders")
            }
        }
    }
}

impl std::error::Error for OrderBuildError {}

/// Builder for constructing [`Order`] instances with validation.
///
/// # Type Safety
///
/// This builder uses *runtime* validation rather than typestate
/// (see the `state` module for a compile-time typestate example).
/// Runtime validation is chosen here because the number of valid
/// state combinations is large and would explode the type space.
///
/// # Example
///
/// ```rust
/// use quant_patterns::builder::{Builder, OrderBuilder, Side, OrderType};
///
/// let order = OrderBuilder::new()
///     .symbol("TSLA")
///     .quantity(50)
///     .side(Side::Sell)
///     .order_type(OrderType::Market)
///     .build()
///     .unwrap();
///
/// assert_eq!(order.symbol(), "TSLA");
/// assert_eq!(order.quantity(), 50);
/// ```
#[derive(Debug, Default)]
pub struct OrderBuilder {
    symbol: Option<String>,
    quantity: Option<u64>,
    side: Option<Side>,
    order_type: Option<OrderType>,
    limit_price: Option<f64>,
    stop_price: Option<f64>,
    time_in_force: Option<TimeInForce>,
}

impl OrderBuilder {
    /// Create a new empty order builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the trading symbol (required).
    pub fn symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    /// Set the quantity (required, must be > 0).
    pub fn quantity(mut self, quantity: u64) -> Self {
        self.quantity = Some(quantity);
        self
    }

    /// Set the side (required).
    pub fn side(mut self, side: Side) -> Self {
        self.side = Some(side);
        self
    }

    /// Set the order type (required).
    pub fn order_type(mut self, order_type: OrderType) -> Self {
        self.order_type = Some(order_type);
        self
    }

    /// Set the limit price (required for Limit, StopLimit).
    pub fn limit_price(mut self, price: f64) -> Self {
        self.limit_price = Some(price);
        self
    }

    /// Set the stop price (required for Stop, StopLimit).
    pub fn stop_price(mut self, price: f64) -> Self {
        self.stop_price = Some(price);
        self
    }

    /// Set time-in-force (defaults to Gtc if not specified).
    pub fn time_in_force(mut self, tif: TimeInForce) -> Self {
        self.time_in_force = Some(tif);
        self
    }
}

impl Builder<Order> for OrderBuilder {
    type Error = OrderBuildError;

    fn build(self) -> Result<Order, Self::Error> {
        let symbol = self.symbol.ok_or(OrderBuildError::MissingSymbol)?;
        let quantity = self.quantity.ok_or(OrderBuildError::InvalidQuantity)?;
        if quantity == 0 {
            return Err(OrderBuildError::InvalidQuantity);
        }
        let side = self.side.ok_or(OrderBuildError::InvalidQuantity)?;
        let order_type = self.order_type.ok_or(OrderBuildError::InvalidQuantity)?;

        // Validate price combinations based on order type.
        match order_type {
            OrderType::Market => {
                if self.limit_price.is_some() {
                    return Err(OrderBuildError::UnexpectedLimitPrice);
                }
                if self.stop_price.is_some() {
                    return Err(OrderBuildError::UnexpectedStopPrice);
                }
            }
            OrderType::Limit => {
                if self.limit_price.is_none() {
                    return Err(OrderBuildError::MissingLimitPrice);
                }
                if self.stop_price.is_some() {
                    return Err(OrderBuildError::UnexpectedStopPrice);
                }
            }
            OrderType::Stop => {
                if self.stop_price.is_none() {
                    return Err(OrderBuildError::MissingStopPrice);
                }
                if self.limit_price.is_some() {
                    return Err(OrderBuildError::UnexpectedLimitPrice);
                }
            }
            OrderType::StopLimit => {
                if self.limit_price.is_none() {
                    return Err(OrderBuildError::MissingLimitPrice);
                }
                if self.stop_price.is_none() {
                    return Err(OrderBuildError::MissingStopPrice);
                }
            }
        }

        Ok(Order {
            symbol,
            quantity,
            side,
            order_type,
            limit_price: self.limit_price,
            stop_price: self.stop_price,
            time_in_force: self.time_in_force.unwrap_or(TimeInForce::Gtc),
        })
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_market_order_ok() {
        let order = OrderBuilder::new()
            .symbol("AAPL")
            .quantity(100)
            .side(Side::Buy)
            .order_type(OrderType::Market)
            .build()
            .unwrap();

        assert_eq!(order.symbol(), "AAPL");
        assert_eq!(order.quantity(), 100);
        assert_eq!(order.side(), Side::Buy);
        assert_eq!(order.order_type(), OrderType::Market);
        assert_eq!(order.time_in_force(), TimeInForce::Gtc);
    }

    #[test]
    fn build_limit_order_ok() {
        let order = OrderBuilder::new()
            .symbol("MSFT")
            .quantity(50)
            .side(Side::Sell)
            .order_type(OrderType::Limit)
            .limit_price(300.00)
            .time_in_force(TimeInForce::Day)
            .build()
            .unwrap();

        assert_eq!(order.limit_price(), Some(300.00));
        assert_eq!(order.time_in_force(), TimeInForce::Day);
    }

    #[test]
    fn build_missing_symbol_fails() {
        let err = OrderBuilder::new()
            .quantity(100)
            .side(Side::Buy)
            .order_type(OrderType::Market)
            .build()
            .unwrap_err();

        assert_eq!(err, OrderBuildError::MissingSymbol);
    }

    #[test]
    fn build_missing_limit_price_fails() {
        let err = OrderBuilder::new()
            .symbol("AAPL")
            .quantity(100)
            .side(Side::Buy)
            .order_type(OrderType::Limit)
            .build()
            .unwrap_err();

        assert_eq!(err, OrderBuildError::MissingLimitPrice);
    }

    #[test]
    fn build_unexpected_limit_price_for_market_fails() {
        let err = OrderBuilder::new()
            .symbol("AAPL")
            .quantity(100)
            .side(Side::Buy)
            .order_type(OrderType::Market)
            .limit_price(150.0)
            .build()
            .unwrap_err();

        assert_eq!(err, OrderBuildError::UnexpectedLimitPrice);
    }

    #[test]
    fn build_zero_quantity_fails() {
        let err = OrderBuilder::new()
            .symbol("AAPL")
            .quantity(0)
            .side(Side::Buy)
            .order_type(OrderType::Market)
            .build()
            .unwrap_err();

        assert_eq!(err, OrderBuildError::InvalidQuantity);
    }
}
