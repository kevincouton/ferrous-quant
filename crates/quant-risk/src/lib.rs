//! Risk metrics and portfolio analytics including VaR, CVaR, rolling statistics, and Sharpe ratio.

#![warn(missing_docs)]

use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use std::collections::VecDeque;

/// Compute Value at Risk (VaR) using historical simulation.
///
/// Returns the percentile-th loss (positive = loss).
pub fn historical_var(returns: &[Decimal], percentile: f64) -> Option<Decimal> {
    if returns.is_empty() || percentile <= 0.0 || percentile >= 1.0 {
        return None;
    }
    let mut sorted = returns.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((1.0 - percentile) * sorted.len() as f64).floor() as usize;
    Some(sorted[idx.min(sorted.len() - 1)])
}

/// Compute Conditional VaR (CVaR / Expected Shortfall).
pub fn historical_cvar(returns: &[Decimal], percentile: f64) -> Option<Decimal> {
    if returns.is_empty() || percentile <= 0.0 || percentile >= 1.0 {
        return None;
    }
    let var = historical_var(returns, percentile)?;
    let tail: Vec<&Decimal> = returns.iter().filter(|r| **r <= var).collect();
    if tail.is_empty() {
        return Some(var);
    }
    let sum: Decimal = tail.iter().copied().sum();
    Some(sum / Decimal::from(tail.len()))
}

/// Rolling window statistics for a return series.
#[derive(Debug, Clone)]
pub struct RollingStats {
    window: VecDeque<Decimal>,
    window_size: usize,
}

impl RollingStats {
    /// Create a new rolling stats calculator.
    pub fn new(window_size: usize) -> Self {
        Self {
            window: VecDeque::with_capacity(window_size),
            window_size,
        }
    }

    /// Push a new return value.
    pub fn push(&mut self, value: Decimal) {
        if self.window.len() >= self.window_size {
            self.window.pop_front();
        }
        self.window.push_back(value);
    }

    /// Current mean.
    pub fn mean(&self) -> Option<Decimal> {
        if self.window.is_empty() {
            return None;
        }
        let sum: Decimal = self.window.iter().sum();
        Some(sum / Decimal::from(self.window.len()))
    }

    /// Current standard deviation (population).
    pub fn std(&self) -> Option<Decimal> {
        let mean = self.mean()?;
        if self.window.len() < 2 {
            return None;
        }
        let variance: Decimal = self
            .window
            .iter()
            .map(|v| {
                let diff = *v - mean;
                diff * diff
            })
            .sum::<Decimal>()
            / Decimal::from(self.window.len());
        variance.sqrt()
    }

    /// Current Sharpe ratio (annualized, assuming daily returns).
    pub fn sharpe_ratio(&self, risk_free_rate: Decimal) -> Option<Decimal> {
        let mean = self.mean()?;
        let std = self.std()?;
        if std == Decimal::ZERO {
            return None;
        }
        // Annualized: multiply by sqrt(252)
        let sqrt_252 = Decimal::from(252).sqrt()?;
        Some((mean - risk_free_rate) / std * sqrt_252)
    }

    /// Current maximum drawdown in the window.
    pub fn max_drawdown(&self) -> Option<Decimal> {
        if self.window.is_empty() {
            return None;
        }
        let mut peak = self.window[0];
        let mut max_dd = Decimal::ZERO;
        for &value in &self.window {
            if value > peak {
                peak = value;
            }
            let dd = (peak - value) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
        Some(max_dd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn var_computes_percentile() {
        let returns: Vec<Decimal> = (1..=100).map(|i| Decimal::from(i)).collect();
        let var = historical_var(&returns, 0.95).unwrap();
        // 95% VaR should be around the 5th percentile
        assert!(var >= Decimal::from(1) && var <= Decimal::from(10));
    }

    #[test]
    fn rolling_mean_and_std() {
        let mut stats = RollingStats::new(3);
        stats.push(Decimal::from(10));
        stats.push(Decimal::from(20));
        stats.push(Decimal::from(30));
        assert_eq!(stats.mean(), Some(Decimal::from(20)));
        assert!(stats.std().is_some());
    }
}
