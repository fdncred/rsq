use chrono::Utc;
use core::hash::{Hash, Hasher};
use std::process;

#[derive(Debug, Clone, Ord, PartialOrd)]
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

impl HistoryItem {
    pub fn new(
        history_id: Option<i64>,
        command_line: String,
        command: String,
        command_params: Option<String>,
        cwd: String,
        duration: i64,
        exit_status: i64,
        session_id: Option<i64>,
        timestamp: chrono::DateTime<Utc>,
        run_count: i64,
    ) -> Self {
        let session_id = session_id.unwrap_or_else(|| process::id().into());

        Self {
            history_id,
            command_line,
            command,
            command_params,
            cwd,
            duration,
            exit_status,
            session_id,
            timestamp,
            run_count,
        }
    }
}

impl PartialEq for HistoryItem {
    // for the sakes of listing unique history only, we do not care about
    // anything else
    // obviously this does not refer to the *same* item of history, but when
    // we only render the command, it looks the same
    fn eq(&self, other: &Self) -> bool {
        self.command == other.command
    }
}

impl Eq for HistoryItem {}

impl Hash for HistoryItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.command.hash(state);
    }
}
