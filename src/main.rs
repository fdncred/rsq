#![allow(dead_code)]
#![allow(unused_variables)]

pub mod database;
pub mod history_item;

use database::{Database, Sqlite};
// use crate::history_item::HistoryItem;
// use chrono::{DateTime, NaiveDate};
use lazy_static::lazy_static;
use log::debug;
// use rusqlite::{config::DbConfig, params, Connection, Result as SqliteError};
use simplelog::*;
// use std::convert::TryInto;
// use std::io::BufRead;
// use std::io::{self, BufReader, Read};
// use std::io::{Seek, SeekFrom};
use std::{fs::File, path::PathBuf};
// use structopt::StructOpt;
use anyhow::Result;

lazy_static! {
    static ref PID: i64 = std::process::id().into();
}

#[derive(Debug)]
struct Person {
    id: i32,
    name: String,
    data: Option<Vec<u8>>,
}

fn main() -> Result<()> {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Debug,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            File::create("my_rust_binary.log").unwrap(),
        ),
    ])
    .unwrap();

    debug!("starting main");

    let db_path = PathBuf::from("C:\\Users\\dschroeder\\source\\repos\\forks\\sql\\rsq\\hizzy3.db");
    let mut sqlite = match Sqlite::new(db_path, database::SqlLogMode::Trace) {
        Ok(r) => r,
        Err(e) => return Err(e), //return Err(anyhow::Error::msg("erro")),
    };

    // // let conn = Connection::open_in_memory()?;
    // let conn = Connection::open("file:history.db")?;
    // conn.execute(
    //     "CREATE TABLE IF NOT EXISTS person (
    //               id              INTEGER PRIMARY KEY,
    //               name            TEXT NOT NULL,
    //               data            BLOB
    //               )",
    //     [],
    // )?;
    // let me = Person {
    //     id: 0,
    //     name: "Steven".to_string(),
    //     data: None,
    // };
    // conn.execute(
    //     "INSERT INTO person (name, data) VALUES (?1, ?2)",
    //     params![me.name, me.data],
    // )?;

    // let mut stmt = conn.prepare("SELECT id, name, data FROM person")?;
    // let person_iter = stmt.query_map([], |row| {
    //     Ok(Person {
    //         id: row.get(0)?,
    //         name: row.get(1)?,
    //         data: row.get(2)?,
    //     })
    // })?;

    // for person in person_iter {
    //     println!("Found person {:?}", person.unwrap());
    // }
    Ok(())
}
