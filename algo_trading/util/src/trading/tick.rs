//! Structs and functions for creating and managing Ticks.  Ticks represent one
//! data point in a timeseries.

use std::fmt::{self, Debug, Formatter};

use serde_json;

#[allow(unused_imports)]
use test;

use transport::query_server::QueryServer;

/// A generic tick.  The data it holds is defined by the user.
pub struct GenTick<T> {
    pub timestamp: u64,
    pub data: T,
}

impl<T> Debug for GenTick<T> where T:Debug {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "GenTick{{ {:?} }}", self.data)
    }
}

impl<T> Clone for GenTick<T> where T:Clone {
    fn clone(&self) -> GenTick<T> {
        GenTick{timestamp: self.timestamp, data: self.data.clone()}
    }
}

/// A traditional tick containing a bid and ask in pips.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tick {
    pub bid: usize,
    pub ask: usize,
    pub timestamp: u64
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SymbolTick {
    pub bid: usize,
    pub ask: usize,
    pub timestamp: u64,
    pub symbol: String
}

impl Tick {
    /// Returns a dummy placeholder tick
    pub fn null() -> Tick {
        Tick {bid: 0, ask: 0, timestamp: 0}
    }

    /// Converts a JSON-encoded String into a Tick
    pub fn from_json_string(s: String) -> Tick {
        serde_json::from_str(s.as_str()).expect("Unable to parse tick from string")
    }

    /// generates a JSON string containing the data of the tick
    pub fn to_json_string(&self, symbol :String) -> String {
        serde_json::to_string(&SymbolTick::from_tick(*self, symbol))
            .expect("Couldn't convert tick to json string")
    }

    pub fn to_csv_row(&self) -> String {
        format!("{}, {}, {}\n", self.timestamp, self.bid, self.ask)
    }

    /// Returns the difference between the bid and the ask
    pub fn spread(&self) -> usize {
        self.bid - self.ask
    }

    /// Returns the average of the bid and ask price
    pub fn mid(&self) -> usize {
        (self.bid + self.ask) / 2usize
    }

    /// Saves the tick in the database.  The table "ticks_SYMBOL" must exist.
    pub fn store(&self, symbol: &str, qs: &mut QueryServer) {
        let query = format!(
            "INSERT INTO ticks_{} (tick_time, bid, ask) VALUES ({}, {}, {});",
            symbol,
            self.timestamp,
            self.bid,
            self.ask
        );

        // Asynchronously store the tick in the database
        qs.execute(query);
    }

    /// Saves the tick in the specified table.  The table must exist.
    pub fn store_table(&self, table: &str, qs: &mut QueryServer) {
        let query = format!(
            "INSERT INTO {} (tick_time, bid, ask) VALUES ({}, {}, {});",
            table,
            self.timestamp,
            self.bid,
            self.ask
        );

        // Asynchronously store the tick in the database
        qs.execute(query);
    }

    /// Converts a SymbolTick into a Tick, dropping the symbol
    pub fn from_symboltick(st: SymbolTick) -> Tick {
        Tick {
            timestamp: st.timestamp,
            bid: st.bid,
            ask: st.ask
        }
    }

    /// Converts a String in the format "{timestamp}, {bid}, {ask}" into a Tick
    pub fn from_csv_string(s: &str) -> Tick {
        let spl: Vec<&str> = s.split(", ").collect();
        Tick {
            timestamp: u64::from_str_radix(spl[0], 10).unwrap(),
            bid: usize::from_str_radix(spl[1], 10).unwrap(),
            ask: usize::from_str_radix(spl[2].split('\n').collect::<Vec<_>>()[0], 10).unwrap()
        }
    }
}

impl SymbolTick {
    /// creates a SymbolTick given a Tick and a SymbolTick
    pub fn from_tick(tick: Tick, symbol: String) -> SymbolTick {
        SymbolTick {bid: tick.bid, ask: tick.ask, timestamp: tick.timestamp, symbol: symbol}
    }

    /// Converts a JSON-encoded String into a Tick
    pub fn from_json_string(s: String) -> SymbolTick {
        serde_json::from_str(s.as_str()).expect("Unable to parse tick from string")
    }
}

#[bench]
fn from_csv_string(b: &mut test::Bencher) {
    let s = "1476650327123, 123134, 123156\n";
    let mut t = Tick::null();
    let _ = b.iter(|| {
        t = Tick::from_csv_string(s)
    });
}

// parse a JSON String into a Tick
#[bench]
fn json_to_tick(b: &mut test::Bencher) {
    b.iter(|| {
        let s: String = String::from("{\"bid\": 1.123128412, \"ask\": 1.123128402, \"timestamp\": 1471291001837}");
        Tick::from_json_string(s);
    });
}
