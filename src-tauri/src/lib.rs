pub fn app_name() -> &'static str {
    "RouDex"
}

use rusqlite::{Connection, OpenFlags, Row};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

const EVENT_LIMIT: i64 = 200;

#[derive(Debug, Serialize)]
pub struct LocalSnapshot {
    pub state_path: PathBuf,
    pub initialized: bool,
    pub tasks: Vec<Value>,
    pub runs: Vec<Value>,
    pub approvals: Vec<Value>,
    pub errors: Vec<Value>,
    pub error_count: i64,
}

#[derive(Debug, Serialize)]
pub struct RunEvents {
    pub run_id: String,
    pub total: i64,
    pub events: Vec<Value>,
}

// Observing state must never create a database, migrate it, or dispatch work.
fn open_read_only(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("No se pudo leer el estado local: {error}"))?;
    connection
        .busy_timeout(Duration::from_secs(2))
        .map_err(|error| error.to_string())?;
    Ok(connection)
}

fn rows(
    connection: &Connection,
    sql: &str,
    map: impl FnMut(&Row<'_>) -> rusqlite::Result<Value>,
) -> Result<Vec<Value>, String> {
    let mut statement = connection.prepare(sql).map_err(|error| error.to_string())?;
    let values = statement
        .query_map([], map)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(values)
}

fn event(row: &Row<'_>) -> rusqlite::Result<Value> {
    let raw: String = row.get(3)?;
    let payload: Value = serde_json::from_str(&raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(json!({
        "id": row.get::<_, i64>(0)?, "run_id": row.get::<_, String>(1)?,
        "event_type": row.get::<_, String>(2)?, "payload": payload,
        "created_at": row.get::<_, String>(4)?
    }))
}

// Includes protocol errors and terminal failures even when the event name is
// turn.completed. Do not mistake a successful terminal turn for a successful task.
const ERROR_PREDICATE: &str = "event_type LIKE '%error%' OR event_type LIKE '%failed%'
    OR event_type LIKE '%invalid%' OR event_type LIKE '%interrupted%'
    OR event_type LIKE '%blocked%' OR
    (json_valid(payload) AND (json_extract(payload, '$.status') IN ('failed', 'interrupted', 'blocked')
      OR json_extract(payload, '$.turn.status') IN ('failed', 'interrupted')
      OR json_type(payload, '$.error') IN ('object', 'text')
      OR json_type(payload, '$.turn.error') IN ('object', 'text')))";

pub fn local_snapshot(path: &Path) -> Result<LocalSnapshot, String> {
    let mut snapshot = LocalSnapshot {
        state_path: path.to_path_buf(),
        initialized: false,
        tasks: vec![],
        runs: vec![],
        approvals: vec![],
        errors: vec![],
        error_count: 0,
    };
    // A fresh installation has no state; an existing but unreadable/invalid store
    // is an error, not an empty successful response.
    if !path.try_exists().map_err(|error| error.to_string())? {
        return Ok(snapshot);
    }
    let mut connection = open_read_only(path)?;
    let tx = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    snapshot.tasks = rows(
        &tx,
        "SELECT task_id, spec_id, title, status, profile, model, effort, approval,
        (SELECT group_concat(depends_on, ', ') FROM task_dependencies d WHERE d.task_id = t.task_id)
        FROM tasks t ORDER BY spec_id, task_id",
        |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?, "spec_id": row.get::<_, String>(1)?,
                "title": row.get::<_, String>(2)?, "status": row.get::<_, String>(3)?,
                "profile": row.get::<_, String>(4)?, "model": row.get::<_, String>(5)?,
                "effort": row.get::<_, String>(6)?, "approval": row.get::<_, String>(7)?,
                "dependencies": row.get::<_, Option<String>>(8)?
            }))
        },
    )?;
    snapshot.runs = rows(
        &tx,
        "SELECT r.run_id, r.route_id, r.status, r.model, r.effort, r.created_at,
        t.task_id FROM runs r LEFT JOIN tasks t ON t.task_id = r.turn_id ORDER BY r.rowid DESC",
        |row| {
            Ok(json!({
                "run_id": row.get::<_, String>(0)?, "route_id": row.get::<_, String>(1)?,
                "status": row.get::<_, String>(2)?, "model": row.get::<_, String>(3)?,
                "effort": row.get::<_, String>(4)?, "created_at": row.get::<_, String>(5)?,
                "task_id": row.get::<_, Option<String>>(6)?
            }))
        },
    )?;
    snapshot.approvals = rows(
        &tx,
        "SELECT a.id, a.task_id, a.decision, a.reason, a.created_at,
        a.id = (SELECT max(b.id) FROM task_approvals b WHERE b.task_id = a.task_id)
        FROM task_approvals a ORDER BY a.id DESC",
        |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?, "task_id": row.get::<_, String>(1)?,
                "decision": row.get::<_, String>(2)?, "reason": row.get::<_, Option<String>>(3)?,
                "created_at": row.get::<_, String>(4)?, "latest": row.get::<_, bool>(5)?
            }))
        },
    )?;
    snapshot.error_count = tx
        .query_row(
            &format!("SELECT count(*) FROM events WHERE {ERROR_PREDICATE}"),
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    snapshot.errors = rows(
        &tx,
        &format!(
            "SELECT id, run_id, event_type, payload, created_at FROM events
        WHERE {ERROR_PREDICATE} ORDER BY id DESC LIMIT {EVENT_LIMIT}"
        ),
        event,
    )?;
    tx.commit().map_err(|error| error.to_string())?;
    snapshot.initialized = true;
    Ok(snapshot)
}

pub fn local_run_events(path: &Path, run_id: &str) -> Result<RunEvents, String> {
    let mut connection = open_read_only(path)?;
    let tx = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    let exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM runs WHERE run_id = ?1)",
            [run_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !exists {
        return Err("La ejecución ya no existe en el estado local.".into());
    }
    let total = tx
        .query_row(
            "SELECT count(*) FROM events WHERE run_id = ?1",
            [run_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let mut statement = tx
        .prepare(
            "SELECT id, run_id, event_type, payload, created_at FROM
        (SELECT * FROM events WHERE run_id = ?1 ORDER BY id DESC LIMIT ?2) ORDER BY id",
        )
        .map_err(|error| error.to_string())?;
    let events = statement
        .query_map(rusqlite::params![run_id, EVENT_LIMIT], event)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    tx.commit().map_err(|error| error.to_string())?;
    Ok(RunEvents {
        run_id: run_id.into(),
        total,
        events,
    })
}
