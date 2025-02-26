//! Trading conditions are expressions that the Tick Processor evaluates for every received tick.
//! If the condition returns a `TradingAction` when evaluated, that action is executed.

use uuid::Uuid;

use trading::tick::Tick;

pub trait TradingCondition {
    /// Evaulate a new Tick with the condition.  Returns a TradingAction to take or None.
    fn eval(&mut self, t: &Tick) -> Option<TradingAction>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum TradingAction {
    /// Opens an order at market price +-max_range pips.
    MarketOrder {
        symbol: String, long: bool, size: usize, stop: Option<usize>,
        take_profit: Option<usize>, max_range: Option<usize>,
    },
    /// Opens an order at a price equal or better to `entry_price` as soon as possible.
    LimitOrder{
        symbol: String, long: bool, size: usize, stop: Option<usize>,
        take_profit: Option<usize>, entry_price: usize,
    },
    /// Closes `size` units of a position with the specified UUID at the current market rate.
    MarketClose{ uuid: Uuid, size: usize, },
    /// Places an order to close `size` units of a position with the specified UUID.
    LimitClose{ uuid: Uuid, size: usize, exit_price: usize, },
    /// Modifies an order without taking any trading action
    ModifyOrder{ uuid: Uuid, size: usize, entry_price: usize, stop: Option<usize>, take_profit: Option<usize>,},
    /// Modifies a position without taking any trading action.
    ModifyPosition{ uuid: Uuid, stop: Option<usize>, take_profit: Option<usize> },
    /// Attempts to cancel an order
    CancelOrder{ uuid: Uuid },
}
