pub(crate) use chrono::Utc;
pub(crate) use rusqlite::{params, Connection, OptionalExtension};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use serde_json::{Map, Value};
pub(crate) use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};
pub(crate) use tauri::{AppHandle, Manager};
pub(crate) use uuid::Uuid;

mod commands;
mod contracts;
mod database;
mod read_models;
mod repo_registry;
mod runtime;
mod schema;
mod storage;
mod support;
mod tasks;

#[cfg(test)]
mod tests;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    commands::run();
}
