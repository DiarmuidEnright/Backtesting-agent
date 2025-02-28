//! Defines a backtest, which determines what data is sent and the
//! conditions that trigger it to be sent.

use std::sync::mpsc;
use uuid::Uuid;
use {BacktestType, DataSource, DataDest};
use simbroker::SimBrokerSettings;
use tickgrinder_util::transport::tickstream::TickstreamCommand;

/// Contains controls for pausing, resuming, and stopping a backtest as well as
/// some data about it.
pub struct BacktestHandle {
    pub symbol: String,
    pub backtest_type: BacktestType,
    pub data_source: DataSource,
    pub endpoint: DataDest,
    pub handle: mpsc::SyncSender<TickstreamCommand>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializableBacktestHandle {
    pub uuid: Uuid,
    pub symbol: String,
    pub backtest_type: BacktestType,
    pub data_source: DataSource,
    pub endpoint: DataDest,
}

impl From<(Uuid, BacktestHandle)> for SerializableBacktestHandle {
    fn from((uuid, handle): (Uuid, BacktestHandle)) -> Self {
        SerializableBacktestHandle {
            uuid,
            symbol: handle.symbol,
            backtest_type: handle.backtest_type,
            data_source: handle.data_source,
            endpoint: handle.endpoint,
        }
    }
}

/// Contains all the information necessary to start a backtest
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BacktestDefinition {
    pub start_time: Option<u64>,
    /// Stop backtest after timestamp reached or None
    pub max_timestamp: Option<u64>,
    /// Stop backtest after `max_tick_n` ticks have been processed or None
    pub max_tick_n: Option<usize>,
    pub symbol: String,
    pub backtest_type: BacktestType,
    pub data_source: DataSource,
    pub data_dest: DataDest,
    pub broker_settings: SimBrokerSettings,
}

/// Ticks sent to the SimBroker should be re-broadcast to the client.
#[tokio::test]
async fn tick_retransmission() {
    use std::collections::HashMap;
    use futures::StreamExt;
    use tickgrinder_util::trading::tick::Tick;
    use simbroker::*;

    // Create the SimBroker
    let symbol = "TEST".to_string();
    let mut sim_client = SimBrokerClient::init(HashMap::new())
        .await
        .expect("Failed to initialize SimBrokerClient")
        .expect("SimBrokerClient returned an error");

    // Subscribe to ticks from the SimBroker for the test symbol
    let mut tick_stream = sim_client
        .sub_ticks(symbol)
        .expect("Failed to subscribe to ticks");

    let mut received_ticks = Vec::new();
    for _ in 0..10 {
        if let Some(tick_result) = tick_stream.next().await {
            match tick_result {
                Ok(tick) => {
                    println!("Received tick: {:?}", tick);
                    received_ticks.push(tick);
                }
                Err(e) => panic!("Error receiving tick: {:?}", e),
            }
        } else {
            break;
        }
    }
    assert_eq!(
        received_ticks.len(),
        10,
        "Expected 10 ticks, but received {}",
        received_ticks.len()
    );
}
