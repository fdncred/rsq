# rusqlite demo

Just a quick demo to show off the history_item struct and database trait files.

## How to run / commands that are supported.

The Hiztery enum (ya, i know it's a dumb name - LOL), looks somewhat like this.

```rust
enum HizteryCmd {
    Insert {},
    Update {},
    Delete {},
    Select {},
    Import {},
    Search {},
    Count {},
    Last {},
    First {},
    Load {},
    Range {},
    Before {},
    All {},
}
```

Most of the functionaly is built around a `HistoryItem`. This is what HistoryItem looks like.

```rust
pub struct HistoryItem {
    /// Primary Key, Unique Id
    pub history_id: Option<i64>,
    /// Entire command line
    pub command_line: String,
    /// Command part of the command line
    pub command: String,
    /// Parameters part of the command line
    pub command_params: Option<String>,
    /// Current working directory
    pub cwd: String,
    /// How long it took to run the command
    pub duration: i64,
    /// The exit status / return status of the command
    pub exit_status: i64,
    /// The pid of the running process
    pub session_id: i64,
    /// When the command was run
    pub timestamp: chrono::DateTime<Utc>,
    /// How many times was this command ran
    pub run_count: i64,
}
```

## Description

| enum   | description                                                                                        | params                   | example                                                        |
| ------ | -------------------------------------------------------------------------------------------------- | ------------------------ | -------------------------------------------------------------- |
| Insert | traditional insert statement                                                                       | --text,--rows_to_insert  | cargo run -- insert --text "happy birthday" --rows_to_insert 5 |
| Update | update a row by id                                                                                 | --id                     | cargo run -- update -i 1                                       |
| Delete | delete a row by id                                                                                 | --id                     | cargo run -- delete -i 3                                       |
| Select | select with max number of unique rows                                                              | --max, --unique          | cargo run -- select -m 5 -u                                    |
| Import | import nushell history file into the db                                                            | --file                   | cargo run -- import --file c:\path\to\nushell\history.txt      |
| Search | search db with searchmode prefix, fulltext, or fuzzy with a row limit and query is the search item | --mode, --limit, --query | cargo run -- search -m "p" -q "code"                           |
| Count  | returns the count of rows in the db                                                                | N/A                      | cargo run -- count                                             |
| Last   | returns the first row                                                                              | N/A                      | cargo run -- first                                             |
| First  | returns the last row                                                                               | N/A                      | cargo run -- last                                              |
| Load   | returns the historyitem at db row index, like when you hit up arrow                                | --id                     | cargo run -- load -i 2800                                      |
| Range  | return historyitems from/to date range                                                             | --from, --to             | cargo run -- range -f "2021-07-21" -t "2021-07-25"             |
| Before | return historyitems from datetime with count limit                                                 | --from, --count          | cargo run -- before -f "2021-07-21" -c 25                      |
| All    | just return everything                                                                             | N/A                      | cargo run -- all                                               |
