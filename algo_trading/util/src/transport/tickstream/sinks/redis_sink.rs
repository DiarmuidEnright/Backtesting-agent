//! Send the output ticks of the backtest through a Redis channel

use redis::{Client, cmd};

use transport::redis::get_client;
use trading::tick::Tick;
use transport::tickstream::TickSink;

pub struct RedisSink {
    pub symbol: String,
    pub tx_channel: String,
    pub client: Client
}

impl TickSink for RedisSink {
    fn tick(&mut self, t: Tick) {
        cmd("PUBLISH")
            .arg(self.tx_channel.clone())
            .arg(t.to_json_string(self.symbol.clone()))
            .execute(&self.client);
    }
}

impl RedisSink {
    pub fn new(symbol: String, tx_channel: String, redis_host: &str) -> RedisSink {
        RedisSink {
            symbol: symbol,
            tx_channel: tx_channel,
            client: get_client(redis_host)
        }
    }
}
