//! Backtester module
//!
//! Plays back market data and executes strategies, providing a simulated broker and
//! account as well as statistics and data about the results of the strategy.

#![feature(rustc_attrs, conservative_impl_trait, associated_consts, custom_derive, test, slice_patterns)]
// #![allow(unused_variables, dead_code,)]

extern crate tickgrinder_util;
extern crate rand;
extern crate futures;
extern crate uuid;
extern crate redis;
extern crate postgres;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;
extern crate test;
#[macro_use]
extern crate from_hashmap;
extern crate simbroker;

mod backtest;

use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::env;
use std::collections::HashMap;
use std::str::FromStr;

use uuid::Uuid;
use futures::Future;
use futures::stream::{Stream, BoxStream};
use serde_json::to_string;

use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::transport::redis::{sub_multiple, get_client};
use tickgrinder_util::transport::commands::*;
use tickgrinder_util::transport::tickstream::*;
use tickgrinder_util::trading::tick::Tick;
use tickgrinder_util::instance::PlatformInstance;
use tickgrinder_util::conf::CONF;
use backtest::*;
use simbroker::*;

lazy_static!{
    static ref NO_BACKTEST: String = String::from("No backtest with that UUID!");
}

/// Starts the backtester module, initializing its interface to the rest of the platform
fn main() {
    let args = env::args().collect::<Vec<String>>();
    let uuid: Uuid;

    match *args.as_slice() {
        [_, ref uuid_str] => {
            uuid = Uuid::parse_str(uuid_str.as_str())
                .expect("Unable to parse Uuid from supplied argument");
        },
        _ => panic!("Wrong number of arguments provided!  Usage: ./tick_processor [uuid] [symbol]"),
    }

    let backtester = Backtester::new(uuid);
    let mut csc = backtester.cs.clone();
    let uuid = backtester.uuid;
    backtester.listen(uuid, &mut csc);
}

/// What kind of method used to time the output of data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BacktestType {
    Fast{delay_ms: usize},
    Live,
}

/// Where to get the data to drive the backtest
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DataSource {
    Flatfile,
    RedisChannel{host: String, channel: String},
    Postgres,
    Random,
}

/// Where to send the backtest's generated data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DataDest {
    RedisChannel{host: String, channel: String},
    Console,
    Null,
    SimBroker{uuid: Uuid}, // Requires that a SimBroker is running on the Backtester in order to work
}

#[derive(Clone)]
struct Backtester {
    pub uuid: Uuid,
    pub cs: CommandServer,
    pub running_backtests: Arc<Mutex<HashMap<Uuid, BacktestHandle>>>,
    pub simbrokers: Arc<Mutex<HashMap<Uuid, SimBrokerClient>>>,
}

impl PlatformInstance for Backtester {
    fn handle_command(&mut self, cmd: Command) -> Option<Response> {
        match cmd {
            Command::Ping => Some(Response::Pong{ args: vec![self.uuid.hyphenated().to_string()] }),
            Command::Type => Some(Response::Info{ info: String::from("Backtester") }),
            Command::StartBacktest{definition: definition_str} => {
                let definition = serde_json::from_str(&definition_str);
                if definition.is_err() {
                    let err_msg = definition.err().unwrap();
                    Some(Response::Error{
                        status: format!("Can't parse backtest defition from String: {}", err_msg)
                    })
                } else {
                    // start the backtest and register a handle internally
                    let uuid = self.start_backtest(definition.unwrap());

                    Some(match uuid {
                        Ok(uuid) => Response::Info{info: uuid.hyphenated().to_string()},
                        Err(err) => Response::Error{status: err}
                    })
                }
            },
            Command::Kill => {
                thread::spawn(|| {
                    thread::sleep(std::time::Duration::from_secs(3));
                    std::process::exit(0);
                });

                Some(Response::Info{info: String::from("Backtester will self-destruct in 3 seconds.")})
            }
            Command::PauseBacktest{uuid} => {
                Some(match self.send_backtest_cmd(&uuid, TickstreamCommand::Pause) {
                    Ok(()) => Response::Ok,
                    Err(()) => Response::Error{status: NO_BACKTEST.clone()},
                })
            },
            Command::ResumeBacktest{uuid} => {
                Some(match self.send_backtest_cmd(&uuid, TickstreamCommand::Resume) {
                    Ok(()) => Response::Ok,
                    Err(()) => Response::Error{status: NO_BACKTEST.clone()},
                })
            },
            Command::StopBacktest{uuid} => {
                Some(match self.send_backtest_cmd(&uuid, TickstreamCommand::Stop) {
                    Ok(()) => {
                        // deregister from internal running backtest list
                        self.remove_backtest(&uuid);
                        Response::Ok
                    },
                    Err(()) => Response::Error{status: NO_BACKTEST.clone()},
                })
            },
            Command::ListBacktests => {
                let backtests = self.running_backtests.lock().unwrap();
                let mut message_vec = Vec::new();
                for (uuid, backtest) in backtests.iter() {
                    let ser_handle = SerializableBacktestHandle::from_handle(backtest, *uuid);
                    message_vec.push(ser_handle);
                }

                let message = to_string(&message_vec);
                Some(match message {
                    Ok(msg) => Response::Info{ info: msg },
                    Err(e) => Response::Error{ status: format!("Unable to convert backtest list into String: {:?}", e) },
                })
            },
            Command::SpawnSimbroker{settings} => {
                let uuid = self.init_simbroker(settings);
                Some(Response::Info{info: uuid.hyphenated().to_string()})
            },
            Command::ListSimbrokers => {
                let simbrokers = self.simbrokers.lock().unwrap();
                let mut uuids = Vec::new();
                for (uuid, _) in simbrokers.iter() {
                    uuids.push(uuid.hyphenated().to_string());
                }
                let message = serde_json::to_string(&uuids).unwrap();
                Some(Response::Info{info: message})
            },
            _ => Some(Response::Error{ status: String::from("Backtester doesn't recognize that command.") })
        }
    }
}

impl Backtester {
    pub fn new(uuid: Uuid) -> Backtester {
        Backtester {
            uuid: uuid,
            cs: CommandServer::new(uuid, "Backtester"),
            running_backtests: Arc::new(Mutex::new(HashMap::new())),
            simbrokers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Creates a SimBroker that's managed by the Backtester.  Returns its UUID.
    pub fn init_simbroker(&mut self, settings: HashMap<String, String>) -> Uuid {
        let mut simbrokers = self.simbrokers.lock().unwrap();
        // TODO: Use new updated SimbrokerSettings
        let simbroker = SimBrokerClient::init(settings).wait().unwrap().unwrap();
        let uuid = Uuid::new_v4();
        simbrokers.insert(uuid, simbroker);
        uuid
    }

    /// Initiates a new backtest and adds it to the internal list of monitored backtests.
    fn start_backtest(
        &mut self, definition: BacktestDefinition) -> Result<Uuid, String>
    {
        let msg = format!("Starting backtest with definition: {:?}", definition);
        self.cs.notice(None, &msg);
        // Create the TickGenerator that provides the backtester with data
        let mut src: Box<TickGenerator> = resolve_data_source(
            &definition.data_source, definition.symbol.clone(), definition.start_time
        );

        // create channel for communicating messages to the running backtest sent externally
        let (external_handle_tx, handle_rx) = mpsc::sync_channel::<TickstreamCommand>(5);
        // create channel for communicating messages to the running backtest internally
        let internal_handle_tx = external_handle_tx.clone();

        // modify the source tickstream to add delay between the ticks or add some other kind of
        // advanced functionality to the way they're outputted
        let tickstream: Result<BoxStream<Tick, ()>, String> = match definition.backtest_type {
            BacktestType::Fast{delay_ms} => src.get(
                Box::new(FastMap{delay_ms: delay_ms}), handle_rx
            ),
            BacktestType::Live => src.get(Box::new(LiveMap::new()), handle_rx),
        };

        if tickstream.is_err() {
            return Err( format!("Error creating tickstream: {}", tickstream.err().unwrap()) )
        }

        // create a TickSink that receives the output of the backtest
        let dst_opt: Result<Box<TickSink + Send>, Uuid> = match definition.data_dest {
            DataDest::RedisChannel{ref host, ref channel} => {
                Ok(Box::new(RedisSink::new(definition.symbol.clone(), channel.clone(), host.as_str())))
            },
            DataDest::Console => Ok(Box::new(ConsoleSink{})),
            DataDest::Null => Ok(Box::new(NullSink{})),
            DataDest::SimBroker{uuid} => Err(uuid),
        };

        let _definition = definition.clone();
        let mut i = 0;
        let uuid = Uuid::new_v4();

        // initiate tick flow
        let mut csc = self.cs.clone();
        if dst_opt.is_ok() {
            let mut dst = dst_opt.unwrap();
            thread::spawn(move || {
                for t_res in tickstream.unwrap().wait() {
                    match t_res {
                        Ok(t) => {
                            i += 1;

                            // send the tick to the sink
                            dst.tick(t);

                            if check_early_exit(&t, &_definition, i) {
                                let msg = "Backtest early exit condition true; exiting backtest.";
                                csc.notice(None, msg);
                                return Err(())
                            }
                        },
                        Err(_) => {
                            csc.notice(None, "Stopping backtest because tickstream has ended");
                            internal_handle_tx.send(TickstreamCommand::Stop)
                                .expect("Sending through the internal handle failed; tickstream dropped?");
                        }
                    };
                }
                Ok(())
            });
        } else {
            let mut simbrokers = self.simbrokers.lock().unwrap();
            let simbroker_opt = simbrokers.get_mut(&dst_opt.err().unwrap());
            if simbroker_opt.is_none() {
                return Err("No SimBroker running with that Uuid!".to_string())
            }

            let simbroker = simbroker_opt.unwrap();
            // plug the tickstream into the matching SimBroker
            // TODO TODO TODO: Implement proper values here
            // BIG TODO; a large part of the backtester logic will have to be re-thought
            // simbroker.register_tickstream(definition.symbol.clone(), tickstream.unwrap(), true, 6).unwrap();
            // simbroker.init_sim_loop().expect("Unable to start SimBroker sim loop");
        }

        let handle = BacktestHandle {
            symbol: definition.symbol,
            backtest_type: definition.backtest_type,
            data_source: definition.data_source,
            endpoint: definition.data_dest,
            handle: external_handle_tx
        };

        // register the backtest's existence
        let mut backtest_list = self.running_backtests.lock().unwrap();
        backtest_list.insert(uuid, handle);

        Ok(uuid)
    }

    /// Removes a stopped backtest from the internal running backtest list
    pub fn remove_backtest(&mut self, uuid: &Uuid) {
        let mut handles = self.running_backtests.lock().unwrap();
        handles.remove(uuid);
    }

    /// Sends a command to a managed backtest
    pub fn send_backtest_cmd(&mut self, uuid: &Uuid, cmd: TickstreamCommand) -> Result<(), ()> {
        let handles = self.running_backtests.lock().unwrap();
        let handle = handles.get(uuid);

        if handle.is_none() {
            return Err(());
        }
        let sender: &mpsc::SyncSender<TickstreamCommand> = &handle.unwrap().handle;
        sender.send(cmd)
            .expect("The receiver corresponding to the sender in the backtest handle seems to have been dropped.");

        Ok(())
    }
}

/// Creates a `TickGenerator` from a `DataSource` and symbol String
pub fn resolve_data_source(data_source: &DataSource, symbol: String, start_time: Option<u64>) -> Box<TickGenerator> {
    match *data_source {
        DataSource::Flatfile => {
            Box::new(FlatfileReader{
                symbol: symbol.clone(),
                start_time: start_time,
            }) as Box<TickGenerator>
        },
        DataSource::RedisChannel{ref host, ref channel} => {
            Box::new(
                RedisReader::new(symbol.clone(), host.clone(), channel.clone())
            ) as Box<TickGenerator>
        },
        DataSource::Random => {
            Box::new(RandomReader {}) as Box<TickGenerator>
        },
        DataSource::Postgres => {
            Box::new(PostgresReader {symbol: symbol, start_time: start_time} )
        },
    }
}

/// Returns true if the backtest has met a stop condition.
fn check_early_exit (
    t: &Tick, def: &BacktestDefinition, i: usize
) -> bool {
    if (def.max_tick_n.is_some() && def.max_tick_n.unwrap() <= i) ||
            (def.max_timestamp.is_some() && def.max_timestamp.unwrap() <= t.timestamp) {
        return true
    }

    false
}

#[test]
fn backtest_n_early_exit() {
    let rx = tickgrinder_util::transport::redis::sub_channel(CONF.redis_host, "test1_ii");

    let mut bt = Backtester::new(Uuid::new_v4());
    let definition = BacktestDefinition {
        start_time: None,
        max_tick_n: Some(10),
        max_timestamp: None,
        symbol: "TEST".to_string(),
        backtest_type: BacktestType::Fast{delay_ms: 0},
        data_source: DataSource::Random,
        data_dest: DataDest::RedisChannel{
            host: CONF.redis_host.to_string(),
            channel: "test1_ii".to_string()
        },
        broker_settings: SimBrokerSettings::default(),
    };

    let uuid = bt.start_backtest(definition).unwrap();
    // backtest starts paused so resume it
    let _ = bt.send_backtest_cmd(&uuid, TickstreamCommand::Resume);
    let res = rx.wait().take(10).collect::<Vec<_>>();
    assert_eq!(res.len(), 10);
}

#[test]
fn backtest_timestamp_early_exit() {
    let rx = tickgrinder_util::transport::redis::sub_channel(CONF.redis_host, "test2_ii");

    let mut bt = Backtester::new(Uuid::new_v4());
    let definition = BacktestDefinition {
        start_time: None,
        max_tick_n: None,
        max_timestamp: Some(8),
        symbol: "TEST".to_string(),
        backtest_type: BacktestType::Fast{delay_ms: 0},
        data_source: DataSource::Random,
        data_dest: DataDest::RedisChannel{
            host: CONF.redis_host.to_string(),
            channel: "test2_ii".to_string()
        },
        broker_settings: SimBrokerSettings::default(),
    };

    let uuid = bt.start_backtest(definition)
        .expect("start_backtest() returned Err!");
    // backtest starts paused so resume it
    bt.send_backtest_cmd(&uuid, TickstreamCommand::Resume).expect("no handle exists for the backtest!");
    let res = rx.wait().take(8).collect::<Vec<_>>();
    assert_eq!(res.len(), 8);
}
