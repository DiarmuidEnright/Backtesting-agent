// functions for communicating with the postgresql database

#![allow(unused_must_use)]

use postgres::{Connection, Error, TlsMode};

use conf::CONF;

pub fn get_client() -> Result<Connection, Error> {
    let conn_string = format!("postgres://{}:{}@{}:{}/{}",
        CONF.postgres_user,
        CONF.postgres_password,
        CONF.postgres_host,
        CONF.postgres_port,
        CONF.postgres_db
    );

    // TODO: Look into setting up TLS
    Connection::connect(conn_string.as_str(), TlsMode::None)
}

/**************************\
*  TICK-RELATED FUNCTIONS  *
\**************************/

pub fn csv_to_tick_table(filename: &str, table_name: &str, client: &Connection) {
    let query = format!(
        "COPY {}(tick_time, bid, ask) FROM {} DELIMETER ', ' CSV HEADER",
        table_name,
        filename
    );
    client.execute(&query, &[]);
}

/// Creates a new table for ticks with given symbol if such a table doesn't already exist.
pub fn init_tick_table(symbol: &str, client: &Connection, pg_user: &str) -> Result<(), String> {
    tick_table_inner(format!("ticks_{}", symbol).as_str(), client, pg_user)
}

/// Initializes a table in which historical ticks can be stored if such a table doesn't already exist.
pub fn init_hist_data_table(table_name: &str, client: &Connection, pg_user: &str) -> Result<(), String> {
    tick_table_inner(table_name, client, pg_user)
}

fn tick_table_inner(table_name: &str, client: &Connection, pg_user: &str) -> Result<(), String> {
    let query1 = format!(
    "CREATE TABLE IF NOT EXISTS {}
    (
      tick_time BIGINT NOT NULL PRIMARY KEY UNIQUE,
      bid BIGINT NOT NULL,
      ask BIGINT NOT NULL
    )
    WITH (
      OIDS=FALSE
    );", table_name);
    let query2 = format!(
    "ALTER TABLE {}
      OWNER TO {};", table_name, pg_user);
    client.execute(&query1, &[])
        .map_err(|_| "Error while querying postgres to set up tick table" );
    client.execute(&query2, &[])
        .map_err(|_| "Error while querying postgres to set up tick table" );

    Ok(())
}

/***************************
* ADMINISTRATIVE FUNCTIONS *
***************************/

/// Drops all tables in the database, resetting it to defaults
pub fn reset_db(client: &Connection, pg_user: &'static str) -> Result<(), Error> {
    let query = format!("DROP SCHEMA public CASCADE;
        CREATE SCHEMA public AUTHORIZATION {};
        ALTER SCHEMA public OWNER TO {};
        GRANT ALL ON SCHEMA public TO {};",
            pg_user, pg_user, pg_user);
    client.batch_execute(query.as_str())
}
