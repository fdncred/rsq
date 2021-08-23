use crate::history_item::HistoryItem;
use chrono::prelude::{DateTime, TimeZone};
use chrono::Utc;
use itertools::Itertools;
use log::debug;
use rusqlite::{params, Connection, Row, Transaction};
use std::time::{Duration, UNIX_EPOCH};
// use std::error::Error;
use anyhow::Result;
use log::info;
use std::path::Path;
// use std::str::FromStr;

// #[derive(Debug)]
// enum SqliteError {
//     SomeError,
//     SqliteError(err),
// }

// impl From<rusqlite::Error> for SqliteError {
//     fn from(e: rusqlite::Error) -> Self {
//         SqliteError::SqliteError(e.to_string())
//     }
// }

pub trait Database {
    fn save(&mut self, h: &HistoryItem) -> Result<()>;
    fn save_bulk(&mut self, h: &[HistoryItem]) -> Result<()>;
    fn load(&self, id: &str) -> Result<HistoryItem>;
    fn list(&self, max: Option<usize>, unique: bool) -> Result<Vec<HistoryItem>>;
    fn range(
        &self,
        from: chrono::DateTime<Utc>,
        to: chrono::DateTime<Utc>,
    ) -> Result<Vec<HistoryItem>>;
    fn update(&self, h: &HistoryItem) -> Result<usize>;
    fn history_count(&self) -> Result<i64>;
    fn first(&self) -> Result<HistoryItem>;
    fn last(&self) -> Result<HistoryItem>;
    fn before(&self, timestamp: chrono::DateTime<Utc>, count: i64) -> Result<Vec<HistoryItem>>;
    fn search(
        &self,
        limit: Option<i64>,
        search_mode: SearchMode,
        query: &str,
    ) -> Result<Vec<HistoryItem>>;
    fn query_history(&self, query: &str) -> Result<Vec<HistoryItem>>;
    fn delete_history_item(&self, id: i64) -> Result<u64>;
}

pub struct Sqlite {
    conn: Connection,
    sql_log_mode: SqlLogMode,
}

impl Sqlite {
    pub fn new(path: impl AsRef<Path>, sql_log_mode: SqlLogMode) -> Result<Self> {
        let path = path.as_ref();
        debug!("opening sqlite database at {:?}", path);

        let create = !path.exists();
        if create {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
        }

        //TODO: Investigate
        // https://github.com/ivanceras/r2d2-sqlite

        let mut conn = Connection::open(format!("file:{}", path.as_os_str().to_str().unwrap()))?;
        set_log_mode(&mut conn, sql_log_mode);

        //https://sqlite.org/pragma.html#pragma_journal_mode
        //https://www.sqlite.org/pragma.html#pragma_busy_timeout - no clue if 1000 is right
        conn.execute_batch(
            "
            PRAGMA page_size=32768;
            PRAGMA journal_mode=wal;
            PRAGMA wal_autocheckpoint=32;
            PRAGMA journal_size_limit=3145728;
            PRAGMA foreign_keys=ON;
            PRAGMA busy_timeout = 1000;
            ",
        )?;

        Self::setup_db(&conn)?;
        Ok(Self { conn, sql_log_mode })
    }

    fn setup_db(conn: &Connection) -> Result<usize> {
        debug!("running sqlite database setup");

        let history_table = r#"
        CREATE TABLE IF NOT EXISTS history_items (
            history_id     INTEGER PRIMARY KEY NOT NULL,
            timestamp      INTEGER NOT NULL,
            duration       INTEGER NOT NULL,
            exit_status    INTEGER NOT NULL,
            command_line   TEXT NOT NULL,
            command        TEXT NOT NULL,
            command_params TEXT NOT NULL,
            cwd            TEXT NOT NULL,
            session_id     INTEGER NOT NULL,
            run_count      INTEGER NOT NULL,

            UNIQUE(timestamp, cwd, command)
        );

        CREATE INDEX IF NOT EXISTS idx_history_timestamp on history_items(timestamp);
        CREATE INDEX IF NOT EXISTS idx_history_command on history_items(command);"#;

        let performance_table = r#"
        CREATE TABLE IF NOT EXISTS performance_items (
            perf_id     INTEGER NOT NULL PRIMARY KEY,
            metrics     FLOAT NOT NULL,
            history_id  INTEGER NOT NULL
            REFERENCES history_items(history_id) ON DELETE CASCADE ON UPDATE CASCADE
          );
        "#;

        conn.execute(history_table, [])?;
        Ok(conn.execute(performance_table, [])?)
    }

    fn save_raw(tx: &mut Transaction, h: &HistoryItem) -> Result<usize> {
        // history_id,
        // command_line,
        // command,
        // command_params,
        // cwd,
        // duration,
        // exit_status,
        // session_id,
        // timestamp,
        // run_count,

        let cmd_params = match h.command_params.as_ref() {
            Some(p) => &p,
            None => "",
        };

        // We don't need the history_id here because it's an auto number field
        // so it should be ever increasing
        Ok(tx.execute(
            "insert or ignore into history_items (command_line, command, command_params, cwd, duration, exit_status, session_id, timestamp, run_count) values(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![h.command_line.as_str(), h.command.as_str(), cmd_params, h.cwd.as_str(), h.duration, h.exit_status, h.session_id, h.timestamp.timestamp_nanos(), h.run_count]
        )?)
    }

    fn convert_time(h: &HistoryItem) {
        // example of how to convert timestamp_nanos() to regular time
        // use chrono::prelude::DateTime;
        // use chrono::Utc;
        // use std::time::{Duration, UNIX_EPOCH};

        // Creates a new SystemTime from the specified number of whole seconds
        let d = UNIX_EPOCH + Duration::from_nanos(1626813332831940400);
        // Create DateTime from SystemTime
        let datetime = DateTime::<Utc>::from(d);

        // I'm not sure there's a way to confidently split up a timestamp
        // let dt = NaiveDateTime::from_timestamp(1626813332, 831940400);
        // println!("NDT {}", dt.format("%Y-%m-%d %H:%M:%S.%f").to_string());

        // Formats the combined date and time with the specified format string.
        let timestamp_str = datetime.format("%Y-%m-%d %H:%M:%S.%f").to_string();
        println! {"{}",timestamp_str};
    }

    fn query_history(row: &Row) -> Result<HistoryItem> {
        Ok(HistoryItem {
            history_id: row.get("history_id")?,
            command_line: row.get("command_line")?,
            command: row.get("command")?,
            command_params: row.get("command_params")?,
            cwd: row.get("cwd")?,
            duration: row.get("duration")?,
            exit_status: row.get("exit_status")?,
            session_id: row.get("session_id")?,
            timestamp: Utc.timestamp_nanos(row.get("timestamp")?),
            run_count: row.get("run_count")?,
        })
    }
}

impl std::ops::Deref for Sqlite {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        &self.conn
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SqlLogMode {
    /// Logging is disabled
    Disabled,
    /// Records timings for each SQL statement
    Profile,
    /// Prints all executed SQL statements
    Trace,
}

impl SqlLogMode {
    pub fn variants() -> [&'static str; 3] {
        ["disabled", "profile", "trace"]
    }
}

impl Default for SqlLogMode {
    fn default() -> Self {
        Self::Disabled
    }
}

impl core::str::FromStr for SqlLogMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "disabled" => Ok(Self::Disabled),
            "profile" => Ok(Self::Profile),
            "trace" => Ok(Self::Trace),
            _ => Err("Could not parse SqlLogMode"),
        }
    }
}

impl ToString for SqlLogMode {
    fn to_string(&self) -> String {
        match self {
            SqlLogMode::Disabled => "disabled",
            SqlLogMode::Profile => "profile",
            SqlLogMode::Trace => "trace",
        }
        .into()
    }
}

// These callbacks use info logging because they are never enabled by default,
// only when explicitly turned on via CLI arguments or interactive CLI commands.
// Setting them to anything other than info would remove the ability to get SQL
// logging from a running server that wasn't started at higher than info.
fn rusqlite_trace_callback(log_message: &str) {
    info!("{}", log_message);
}
fn rusqlite_profile_callback(log_message: &str, dur: Duration) {
    info!("{} Duration: {:?}", log_message, dur);
}

// from veloren
fn set_log_mode(connection: &mut Connection, sql_log_mode: SqlLogMode) {
    // Rusqlite's trace and profile logging are mutually exclusive and cannot be
    // used together
    match sql_log_mode {
        SqlLogMode::Trace => {
            connection.trace(Some(rusqlite_trace_callback));
        }
        SqlLogMode::Profile => {
            connection.profile(Some(rusqlite_profile_callback));
        }
        SqlLogMode::Disabled => {
            connection.trace(None);
            connection.profile(None);
        }
    };
}

impl Database for Sqlite {
    fn save(&mut self, h: &HistoryItem) -> Result<()> {
        debug!("saving history to sqlite");

        let mut tx = self.conn.transaction()?;
        Self::save_raw(&mut tx, h)?;
        Ok(tx.commit()?)
    }

    fn save_bulk(&mut self, h: &[HistoryItem]) -> Result<()> {
        debug!("saving history to sqlite");

        let mut tx = self.conn.transaction()?;

        for i in h {
            Self::save_raw(&mut tx, i)?;
        }

        Ok(tx.commit()?)
    }

    fn load(&self, id: &str) -> Result<HistoryItem> {
        debug!("loading history item {}", id);

        // let res = conn
        //     .execute("select * from history_items where history_id = ?1")
        //     .bind(id)
        //     .map(Self::query_history)
        //     .fetch_one(&self.pool)
        //     .await?;

        let mut stmt = self
            .conn
            .prepare("select * from history_items where history_id = ?1")?;
        // let mut rows = stmt.query(params![id])?;
        // let arow = match rows.next()? {
        //     Some(r) => *r,
        //     None => return Err(anyhow::Error::msg("blah")),
        // };
        // let row = Self::query_history(arow)?;

        let ro = stmt.query_row(params![id], |r| Ok(Self::query_history(r)))?;
        ro

        // let mut rows = stmt.query(params![id])?;
        // // let rows = rows.map(|r| Ok(Self::query_history(r))).collect();
        // // let row = rows.and_then(|f| Self::query_history(*f)).collect();
        // // let row = Self::query_history(rows)?;

        // // Ok(row)
        // if let Some(row) = rows.next()? {
        //     return Self::query_history(row);
        // } else {
        //     Err(anyhow::Error::msg("blah"))
        // }
    }

    fn update(&self, h: &HistoryItem) -> Result<usize> {
        debug!("updating sqlite history");
        debug!("history_item = [{:?}]", &h);

        // conn.execute(
        //     "update history_items
        //         set timestamp = ?2, duration = ?3, exit_status = ?4, command = ?5, cwd = ?6, session_id = ?7
        //         where history_id = ?1",
        // )
        // .bind(h.history_id)
        // .bind(h.timestamp.timestamp_nanos())
        // .bind(h.duration)
        // .bind(h.exit_status)
        // .bind(h.command.as_str())
        // .bind(h.cwd.as_str())
        // .bind(h.session_id)
        // .execute(&self.pool)
        // .await?;

        let cmd_params = match h.command_params.as_ref() {
            Some(p) => &p,
            None => "",
        };

        Ok(self.conn.execute(
            "update history_items
            set command_line = ?1, command = ?2, command_params = ?3, cwd = ?4, duration = ?5,
            exit_status = ?6, session_id = ?7, timestamp = ?8, run_count = 9?",
            params![
                h.command_line.as_str(),
                h.command.as_str(),
                cmd_params,
                h.cwd.as_str(),
                h.duration,
                h.exit_status,
                h.session_id,
                h.timestamp.timestamp_nanos(),
                h.run_count
            ],
        )?)
    }

    // make a unique list, that only shows the *newest* version of things
    fn list(&self, max: Option<usize>, unique: bool) -> Result<Vec<HistoryItem>> {
        debug!("listing history");

        // very likely vulnerable to SQL injection
        // however, this is client side, and only used by the client, on their
        // own data. They can just open the db file...
        // otherwise building the query is awkward
        let query = format!(
            "select * from history_items h
                {}
                order by timestamp desc
                {}",
            // inject the unique check
            if unique {
                "where timestamp = (
                        select max(timestamp) from history_items
                        where h.command = history_items.command
                    )"
            } else {
                ""
            },
            // inject the limit
            if let Some(max) = max {
                format!("limit {}", max)
            } else {
                "".to_string()
            }
        );

        // let res = conn
        //     .execute(query.as_str())
        //     .map(Self::query_history)
        //     .fetch_all(&self.pool)
        //     .await?;

        // Ok(res)
        let mut hist_rows: Vec<HistoryItem> = Vec::new();
        let mut stmt = self.conn.prepare(query.as_str())?;

        let rows = stmt.query_map([], |row| {
            let res = Self::query_history(row);
            hist_rows.push(res.unwrap());
            Ok(())
        })?;

        // let rows = stmt.query([])?;
        // rows.map(|r| {
        //     let res = Self::query_history(r);
        //     hist_rows.push(res.unwrap());
        //     Ok(())
        // });

        Ok(hist_rows)
    }

    fn range(
        &self,
        from: chrono::DateTime<Utc>,
        to: chrono::DateTime<Utc>,
    ) -> Result<Vec<HistoryItem>> {
        debug!("listing history from {:?} to {:?}", from, to);

        // let res = conn.execute(
        //     "select * from history_items where timestamp >= ?1 and timestamp <= ?2 order by timestamp asc",
        // )
        // .bind(from.timestamp_nanos())
        // .bind(to.timestamp_nanos())
        //     .map(Self::query_history)
        // .fetch_all(&self.pool)
        // .await?;

        // let mut rows = stmt.query(params![id])?;
        let mut hist_rows: Vec<HistoryItem> = Vec::new();

        let mut stmt = self.conn.prepare("select * from history_items where timestamp >= ?1 and timestamp <= ?2 order by timestamp asc")?;
        // let rows = stmt.query(params![from.timestamp_nanos(), to.timestamp_nanos()])?;
        let rows = stmt.query_map(
            params![from.timestamp_nanos(), to.timestamp_nanos()],
            |row| {
                let res = Self::query_history(row);
                hist_rows.push(res.unwrap());
                Ok(())
            },
        )?;

        Ok(hist_rows)
    }

    fn history_count(&self) -> Result<i64> {
        // let res: (i64,) = conn
        //     .execute_as("select count(1) from history_items")
        //     .fetch_one(&self.pool)
        //     .await?;

        // Ok(res.0)

        let mut stmt = self.conn.prepare("select count(1) from history_items")?;

        // let ro = stmt.query_row(params![id], |r| Ok(Self::query_history(r)))?;
        // ro
        let cnt = stmt.query_row([], |r| r.get(0))?;
        Ok(cnt)
    }

    fn first(&self) -> Result<HistoryItem> {
        // let res = conn
        //     .execute(
        //         "select * from history_items where duration >= 0 order by timestamp asc limit 1",
        //     )
        //     .map(Self::query_history)
        //     .fetch_one(&self.pool)
        //     .await?;

        // Ok(res)

        let mut stmt = self.conn.prepare(
            "select * from history_items where duration >= 0 order by timestamp asc limit 1",
        )?;

        let row = stmt.query_row([], |r| Ok(Self::query_history(r)))?;
        row
    }

    fn last(&self) -> Result<HistoryItem> {
        // let res = conn
        //     .execute(
        //         "select * from history_items where duration >= 0 order by timestamp desc limit 1",
        //     )
        //     .map(Self::query_history)
        //     .fetch_one(&self.pool)
        //     .await?;

        // Ok(res)
        let mut stmt = self.conn.prepare(
            "select * from history_items where duration >= 0 order by timestamp desc limit 1",
        )?;

        let row = stmt.query_row([], |r| Ok(Self::query_history(r)))?;
        row
    }

    fn before(&self, timestamp: chrono::DateTime<Utc>, count: i64) -> Result<Vec<HistoryItem>> {
        // let res = conn
        //     .execute(
        //         "select * from history_items where timestamp < ?1 order by timestamp desc limit ?2",
        //     )
        //     .bind(timestamp.timestamp_nanos())
        //     .bind(count)
        //     .map(Self::query_history)
        //     .fetch_all(&self.pool)
        //     .await?;

        // Ok(res)

        let mut hist_rows: Vec<HistoryItem> = Vec::new();

        let mut stmt = self.conn.prepare(
            "select * from history_items where timestamp < ?1 order by timestamp desc limit ?2",
        )?;
        let rows = stmt.query_map(params![timestamp.timestamp_nanos(), count], |row| {
            let res = Self::query_history(row);
            hist_rows.push(res.unwrap());
            Ok(())
        })?;

        Ok(hist_rows)
    }

    fn search(
        &self,
        limit: Option<i64>,
        search_mode: SearchMode,
        query: &str,
    ) -> Result<Vec<HistoryItem>> {
        let query = query.to_string().replace("*", "%"); // allow wildcard char
        let limit = limit.map_or("".to_owned(), |l| format!("limit {}", l));

        let query = match search_mode {
            SearchMode::Prefix => query,
            SearchMode::FullText => format!("%{}", query),
            SearchMode::Fuzzy => query.split("").join("%"),
        };

        let mut hist_rows: Vec<HistoryItem> = Vec::new();
        let mut stmt = self.prepare(
            format!(
                "select * from history_items h
            where command like ?1 || '%'
            and timestamp = (
                    select max(timestamp) from history_items
                    where h.command = history_items.command
                )
            order by timestamp desc {}",
                limit.clone()
            )
            .as_str(),
        )?;

        let rows = stmt.query_map([], |row| {
            let res = Self::query_history(row);
            hist_rows.push(res.unwrap());
            Ok(())
        })?;

        Ok(hist_rows)

        // .execute(
        //     format!(
        //         "select * from history_items h
        //     where command like ?1 || '%'
        //     and timestamp = (
        //             select max(timestamp) from history_items
        //             where h.command = history_items.command
        //         )
        //     order by timestamp desc {}",
        //         limit.clone()
        //     )
        //     .as_str(),
        // )
        // .bind(query)
        // .map(Self::query_history)
        // .fetch_all(&self.pool)
        // .await?;

        // Ok(res)
    }

    fn query_history(&self, query: &str) -> Result<Vec<HistoryItem>> {
        // let res = conn
        //     .execute(query)
        //     .map(Self::query_history)
        //     .fetch_all(&self.pool)
        //     .await?;

        // Ok(res)

        let mut hist_rows: Vec<HistoryItem> = Vec::new();
        let mut stmt = self.conn.prepare(query)?;

        let rows = stmt.query_map([], |row| {
            let res = Self::query_history(row);
            hist_rows.push(res.unwrap());
            Ok(())
        })?;

        // let rows = stmt.query([])?;
        // rows.map(|r| {
        //     let res = Self::query_history(r);
        //     hist_rows.push(res.unwrap());
        //     Ok(())
        // });

        Ok(hist_rows)
    }

    fn delete_history_item(&self, id: i64) -> Result<u64> {
        // let res = conn
        //     .execute("delete from history_items where history_id = ?1")
        //     .bind(id)
        //     .execute(&self.pool)
        //     .await?
        //     .rows_affected();
        // Ok(res)

        let mut stmt = self
            .conn
            .prepare("delete from history_items where history_id = ?1")?;
        stmt.execute(params![id])?;
        Ok(1u64)
    }
}

#[derive(Clone, Debug, Copy)]
pub enum SearchMode {
    // #[serde(rename = "prefix")]
    Prefix,

    // #[serde(rename = "fulltext")]
    FullText,

    // #[serde(rename = "fuzzy")]
    Fuzzy,
}

#[cfg(test)]
mod test {
    use super::*;

    fn new_history_item(db: &mut impl Database, cmd: &str) -> Result<()> {
        let history = HistoryItem::new(
            chrono::Local::now(),
            cmd.to_string(),
            "/home/ellie".to_string(),
            0,
            1,
            Some("beep boop".to_string()),
            Some("booop".to_string()),
        );
        return db.save(&history).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    fn test_search_prefix() {
        let mut db = Sqlite::new("sqlite::memory:", SqlLogMode::Disabled)
            .await
            .unwrap();
        new_history_item(&mut db, "ls /home/ellie").await.unwrap();

        let mut results = db.search(None, SearchMode::Prefix, "ls").await.unwrap();
        assert_eq!(results.len(), 1);

        results = db.search(None, SearchMode::Prefix, "/home").await.unwrap();
        assert_eq!(results.len(), 0);

        results = db.search(None, SearchMode::Prefix, "ls  ").await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    fn test_search_fulltext() {
        let mut db = Sqlite::new("sqlite::memory:", SqlLogMode::Disabled)
            .await
            .unwrap();
        new_history_item(&mut db, "ls /home/ellie").await.unwrap();

        let mut results = db.search(None, SearchMode::FullText, "ls").await.unwrap();
        assert_eq!(results.len(), 1);

        results = db
            .search(None, SearchMode::FullText, "/home")
            .await
            .unwrap();
        assert_eq!(results.len(), 1);

        results = db.search(None, SearchMode::FullText, "ls  ").await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    fn test_search_fuzzy() {
        let mut db = Sqlite::new("sqlite::memory:", SqlLogMode::Disabled)
            .await
            .unwrap();
        new_history_item(&mut db, "ls /home/ellie").await.unwrap();
        new_history_item(&mut db, "ls /home/frank").await.unwrap();
        new_history_item(&mut db, "cd /home/ellie").await.unwrap();
        new_history_item(&mut db, "/home/ellie/.bin/rustup")
            .await
            .unwrap();

        let mut results = db.search(None, SearchMode::Fuzzy, "ls /").await.unwrap();
        assert_eq!(results.len(), 2);

        results = db.search(None, SearchMode::Fuzzy, "l/h/").await.unwrap();
        assert_eq!(results.len(), 2);

        results = db.search(None, SearchMode::Fuzzy, "/h/e").await.unwrap();
        assert_eq!(results.len(), 3);

        results = db.search(None, SearchMode::Fuzzy, "/hmoe/").await.unwrap();
        assert_eq!(results.len(), 0);

        results = db
            .search(None, SearchMode::Fuzzy, "ellie/home")
            .await
            .unwrap();
        assert_eq!(results.len(), 0);

        results = db.search(None, SearchMode::Fuzzy, "lsellie").await.unwrap();
        assert_eq!(results.len(), 1);

        results = db.search(None, SearchMode::Fuzzy, " ").await.unwrap();
        assert_eq!(results.len(), 3);
    }
}
