#![allow(dead_code)]
#![allow(unused_variables)]

pub mod database;
pub mod history_item;

use crate::history_item::HistoryItem;
use chrono::{DateTime, NaiveDate};
use database::{Database, SearchMode, Sqlite};
use lazy_static::lazy_static;
use log::debug;
// use rusqlite::{config::DbConfig, params, Connection, Result as SqliteError};
use anyhow::Result;
use simplelog::*;
use std::convert::TryInto;
use std::io::BufRead;
use std::io::{self, BufReader, Read};
use std::io::{Seek, SeekFrom};
use std::{fs::File, path::PathBuf};
use structopt::StructOpt;

lazy_static! {
    static ref PID: i64 = std::process::id().into();
}

#[derive(StructOpt)]
struct Args {
    #[structopt(subcommand)]
    cmd: Option<HizteryCmd>,
}

#[derive(StructOpt)]
#[structopt(about = "sql commands used with history")]
enum HizteryCmd {
    Insert {
        #[structopt(short = "t", long = "text")]
        history_item: String,
        #[structopt(short = "r", long = "rows_to_insert")]
        rows_to_insert: i64,
    },
    Update {
        #[structopt(short = "i", long = "id")]
        history_id: i64,
        // #[structopt(short = "u", long = "update_text")]
        // history_item: String,
    },
    Delete {
        #[structopt(short = "i", long = "id")]
        history_id: i64,
    },
    Select {
        #[structopt(short = "m", long = "max")]
        max: Option<usize>,
        #[structopt(short = "u", long = "unique")]
        unique: bool,
    },
    Import {
        #[structopt(short = "f", long = "file", name = "file path")]
        nushell_history_filepath: String,
    },
    Search {
        #[structopt(short = "m", long = "mode")]
        search_mode: String,
        #[structopt(short = "l", long = "limit")]
        limit: Option<i64>,
        #[structopt(short = "q", long = "query")]
        query: String,
    },
    Count {},
    Last {},
    First {},
    Load {
        #[structopt(short = "i", long = "id")]
        id: String,
    },
    Range {
        #[structopt(short = "f", long = "from")]
        from_date: String,
        #[structopt(short = "t", long = "to")]
        to_date: String,
    },
    Before {
        #[structopt(short = "f", long = "from")]
        from_date: String,
        #[structopt(short = "c", long = "count")]
        count: i64,
    },
    All {},
}

#[paw::main]
fn main(args: Args) -> Result<(), anyhow::Error> {
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
    start(args)?;

    let db_path = PathBuf::from("C:\\Users\\dschroeder\\source\\repos\\forks\\sql\\rsq\\hizzy3.db");
    let sqlite = match Sqlite::new(db_path, database::SqlLogMode::Trace) {
        Ok(r) => r,
        Err(e) => return Err(e), //return Err(anyhow::Error::msg("erro")),
    };

    Ok(())
}

fn start(args: Args) -> Result<(), anyhow::Error> {
    debug!("starting second_attempt");

    let db_path =
        PathBuf::from("C:\\Users\\dschroeder\\source\\repos\\forks\\sql\\hiztery\\hizzy.db");
    let mut sqlite = match Sqlite::new(db_path, database::SqlLogMode::Trace) {
        Ok(r) => r,
        // Err(e) => anyhow::bail!("unexpected error: {}", e),
        Err(e) => return Err(e),
    };

    match args.cmd {
        Some(HizteryCmd::Insert {
            history_item,
            rows_to_insert,
        }) => {
            // cargo run -- insert --text "happy birthday" --rows_to_insert 5
            debug!("Insert with {} {}", &history_item, rows_to_insert);
            for row in 0..rows_to_insert {
                let hi = HistoryItem::new(
                    None,
                    history_item.clone(),
                    "cmd".to_string(),
                    Some("some | param | other_param".to_string()),
                    "path/to/somewhere".to_string(),
                    0,
                    0,
                    Some(*PID),
                    chrono::Utc::now(),
                    1,
                );

                let result = sqlite.save(&hi)?;
                // match result {
                //     Ok(r) => r,
                //     Err(e) => return Err(e),
                // }
            }
        }
        Some(HizteryCmd::Update {
            history_id,
            // history_item,
        }) => {
            // cargo run -- update -i 1
            debug!("Update with id: {}", history_id);
            let hi = HistoryItem::new(
                Some(history_id),
                "cmd_line".to_string(),
                "cmd".to_string(),
                Some("some | updated | command".to_string()),
                "i_updated".to_string(),
                1,
                0,
                Some(*PID),
                chrono::Utc::now(),
                1,
            );

            let result = sqlite.update(&hi)?;
            // match result {
            //     Ok(r) => r,
            //     Err(e) => return Err(e),
            // }
        }
        Some(HizteryCmd::Delete { history_id }) => {
            // cargo run -- delete -i 3
            debug!("Deleting history item: [{}]", history_id);
            let res = sqlite.delete_history_item(history_id)?;
            debug!("Deleted row count: [{}]", res);
        }
        Some(HizteryCmd::Select { max, unique }) => {
            // cargo run -- select -m 5 -u
            debug!("Selecting max: [{:?}] with unique: [{}]", max, unique);
            let output = sqlite.list(max, unique)?;
            for (idx, item) in output.iter().enumerate() {
                debug!("ItemNum: [{}] Row: [{:?}]", idx, item);
            }
        }
        Some(HizteryCmd::Import {
            nushell_history_filepath,
        }) => {
            debug!("Import with file: {}", &nushell_history_filepath);
            let file = File::open(nushell_history_filepath);
            let mut reader = BufReader::new(file.unwrap());
            let lines = count_lines(&mut reader)?;
            debug!("Lines: {}", lines);

            let mut history_vec = vec![];

            for (idx, line) in reader.lines().enumerate() {
                // println!("{}", line?);
                let time = chrono::Utc::now();
                let offset = chrono::Duration::seconds(idx.try_into().unwrap());
                let time = time - offset;

                // self.counter += 1;

                history_vec.push(HistoryItem::new(
                    None,
                    "cmd_line".to_string(),
                    line?.trim_end().to_string(),
                    Some(String::from("unknown")),
                    "some/path".to_string(),
                    -1,
                    0,
                    Some(*PID),
                    time,
                    1,
                ));
            }

            debug!("Preparing for save_bulk");
            let result = sqlite.save_bulk(&history_vec)?;
            let cnt = sqlite.history_count()?;
            //  {
            //     Ok(c) => c,
            //     _ => 0i64,
            // };
            debug!("Imported [{}] history entries", cnt);
        }
        Some(HizteryCmd::Search {
            search_mode,
            limit,
            query,
        }) => {
            // cargo run -- search -m "p" -q "code"
            debug!(
                "Searching with phrase: {}, limit: {:?}, mode: {}",
                &query, limit, &search_mode
            );
            let s_mode = match search_mode.as_ref() {
                "p" => SearchMode::Prefix,
                "f" => SearchMode::FullText,
                "z" => SearchMode::Fuzzy,
                _ => SearchMode::FullText,
            };

            let result = sqlite.search(limit, s_mode, &query);
            match result {
                Ok(r) => {
                    debug!("Found {} hits", r.len());
                    for (idx, hit) in r.iter().enumerate() {
                        debug!("Hit # [{}] History: [{}]", idx + 1, hit.command);
                    }
                }
                _ => debug!("No hits found for phrase: {}", &query),
            }
        }
        Some(HizteryCmd::Count {}) => {
            // cargo run -- count
            debug!("Counting history items.");
            let result = sqlite.history_count()?;
            debug!("Found [{}] history items.", result);
        }
        Some(HizteryCmd::Last {}) => {
            // cargo run -- last
            debug!("Looking for the last history item.");
            let result = sqlite.last()?;
            debug!("Found [{:?}] history items.", result);
        }
        Some(HizteryCmd::First {}) => {
            // cargo run -- first
            debug!("Looking for the first history item.");
            let result = sqlite.first()?;
            debug!("Found [{:?}] history items.", result);
        }
        Some(HizteryCmd::Load { id }) => {
            // cargo run -- load -i 2800
            debug!("Looking for history item [{}].", &id);
            let result = sqlite.load(&id)?;
            debug!("Found [{:?}] history items.", result);
        }
        Some(HizteryCmd::Range { from_date, to_date }) => {
            // cargo run -- range -f "2021-07-21" -t "2021-07-25"
            debug!(
                "Looking for history item between [{}] and [{}].",
                &from_date, &to_date
            );
            let f = NaiveDate::parse_from_str(&from_date, "%Y-%m-%d").unwrap();
            let t = NaiveDate::parse_from_str(&to_date, "%Y-%m-%d").unwrap();
            let f_utc = DateTime::<chrono::Utc>::from_utc(f.and_hms(0, 0, 0), chrono::Utc);
            let t_utc = DateTime::<chrono::Utc>::from_utc(t.and_hms(0, 0, 0), chrono::Utc);
            let result = sqlite.range(f_utc, t_utc)?;

            debug!("Found {} hits", result.len());
            for (idx, hit) in result.iter().enumerate() {
                debug!("Hit # [{}] History: [{:?}]", idx + 1, hit);
            }
        }
        Some(HizteryCmd::Before { from_date, count }) => {
            // cargo run -- before -f "2021-07-21" -c 25
            debug!(
                "Looking for history item after [{}] with max [{}].",
                &from_date, count,
            );
            let f = NaiveDate::parse_from_str(&from_date, "%Y-%m-%d").unwrap();
            let f_utc = DateTime::<chrono::Utc>::from_utc(f.and_hms(0, 0, 0), chrono::Utc);
            let result = sqlite.before(f_utc, count)?;

            debug!("Found {} hits", result.len());
            for (idx, hit) in result.iter().enumerate() {
                debug!("Hit # [{}] History: [{:?}]", idx + 1, hit);
            }
        }
        Some(HizteryCmd::All {}) => {
            // cargo run -- last
            debug!("Looking for all the history items.");
            let result = sqlite.query_history("select * from history_items")?;
            debug!("Found {} hits", result.len());
            for (idx, hit) in result.iter().enumerate() {
                debug!("Hit # [{}] History: [{:?}]", idx + 1, hit);
            }
        }
        None => {}
    }

    Ok(())
}

fn count_lines(buf: &mut BufReader<impl Read + Seek>) -> Result<usize, io::Error> {
    let lines = buf.lines().count();
    buf.seek(SeekFrom::Start(0))?;

    Ok(lines)
}
