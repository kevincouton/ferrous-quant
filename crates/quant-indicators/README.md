# quant-indicators

Technical analysis indicators for financial time series in Rust.

## Indicators

| Indicator | Struct | Description |
|-----------|--------|-------------|
| Simple Moving Average | `Sma` | Arithmetic mean over N periods |
| Exponential Moving Average | `Ema` | Weighted mean with exponential decay |
| Relative Strength Index | `Rsi` | Momentum oscillator (0-100) |
| Bollinger Bands | `BollingerBands` | Volatility bands around SMA |

All indicators implement the `Indicator` trait for uniform usage:

```rust,ignore
use quant_indicators::{Indicator, Sma};
use quant_core::ohlcv::Bar;

let mut sma = Sma::new(20);
// for bar in bars { sma.update(&bar); ... }
```
