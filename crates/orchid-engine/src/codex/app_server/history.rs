//! Native stored-thread operations shared by product import and continuation.
use super::client;
use crate::contracts::ports::RuntimePortError;
use serde_json::{json, Value};
use std::path::PathBuf;

pub fn read_thread(program: &str, home: PathBuf, id: &str) -> Result<Value, RuntimePortError> {
    client::with_connection(program, home, None, |rpc| {
        rpc.call("thread/read", json!({"threadId":id,"includeTurns":true}))
    })
}

pub fn fork_thread(program: &str, home: PathBuf, id: &str, last: &str, cwd: &str) -> Result<Value, RuntimePortError> {
    client::with_connection(program, home, Some(PathBuf::from(cwd)), |rpc| {
        rpc.call("thread/fork", json!({"threadId":id,"lastTurnId":last,"cwd":cwd}))
    })
}
