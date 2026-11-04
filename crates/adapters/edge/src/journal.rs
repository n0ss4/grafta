//! Edits that survive a restart.
//!
//! An edit lives in memory until `save_as`. A toolkit built with
//! [`Toolkit::with_journal`] also appends every call that edited a file to
//! that file's journal, synced before the call returns, and replays the
//! journal when the file is opened again.
//!
//! The calls are replayed rather than their effects stored, because grafta is
//! deterministic about what a call does to given bytes: ids are the highest
//! grafta id plus one, and the layout pass depends only on the file. That is
//! checked, not trusted: each entry carries what the call produced, and the
//! replay stops at the first entry that produces something else.
//!
//! Nothing is thrown away silently. A journal made on other bytes, a journal
//! that cannot be read, and the entries after a replay stopped are all set
//! aside beside the journal, and `open_file` says where.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::app::data::Data;

use crate::port::Journal;
use crate::{fnv, Base, DomainError, Output, Session, ToolError, Toolkit};

/// The journal format. A header with another version is set aside, not read.
const VERSION: u64 = 1;

/// A file's fingerprint as its journal's header writes it.
fn base_json(base: Base) -> Value {
    json!({ "bytes": base.bytes, "fnv": format!("{:016x}", base.fnv) })
}

/// A fingerprint read back from a journal's header.
fn base_from_json(value: &Value) -> Option<Base> {
    Some(Base {
        bytes: value.get("bytes")?.as_u64()?,
        fnv: u64::from_str_radix(value.get("fnv")?.as_str()?, 16).ok()?,
    })
}

/// The journal of a session, named by its handle and the fingerprint of its
/// real path, so two links to one file share a journal.
#[must_use]
pub fn path_for(dir: &Path, session: &Session) -> PathBuf {
    dir.join(format!("{}-{:016x}.jsonl", session.handle, fnv(session.path.to_string_lossy().as_bytes())))
}

/// What a call produced, to compare on replay: how many node changes the file
/// has after it, and the id the result names, when it names one.
fn check(session: &Session, output: &Output) -> Value {
    let mut check = json!({ "nodes": session.raw_index.len() });
    if let Output::Data(value) = output {
        if let Some(id) = value.get("id").and_then(crate::app::data::Data::as_str) {
            check["id"] = json!(id);
        }
    }
    check
}

/// Append a call that edited `handle` to its journal, and sync it. Does
/// nothing for a toolkit that does not journal, during a replay, or for a
/// session that was not read from disk.
pub(crate) fn record(kit: &Toolkit, handle: &str, tool: &str, args: &Value, output: &Output) -> Result<(), ToolError> {
    let (Some(dir), false) = (kit.journal_dir(), kit.replaying()) else { return Ok(()) };
    let session = kit.session(handle)?;
    let Some(base) = session.base else { return Ok(()) };
    let path = path_for(dir, session);
    let mut args = args.clone();
    if let Some(fields) = args.as_object_mut() {
        fields.remove("handle");
    }
    let entry = json!({ "tool": tool, "args": args, "check": check(session, output) });
    session
        .ports
        .journal
        .append(&path, &header(session, base).to_string(), &entry.to_string())
        .map_err(|error| DomainError::NotJournaled { path: path.clone(), error: error.to_string() }.into())
}

fn header(session: &Session, base: Base) -> Value {
    json!({ "grafta_journal": VERSION, "path": session.path.display().to_string(), "base": base_json(base) })
}

/// Remove a session's journal, when its edits are discarded on purpose.
pub(crate) fn discard(dir: Option<&Path>, session: &Session) {
    if let Some(dir) = dir {
        session.ports.journal.remove(&path_for(dir, session));
    }
}

/// The journal as the toolkit's [`Recovery`](crate::Recovery).
pub struct Journaled;

impl crate::Recovery for Journaled {
    fn discard(&self, kit: &Toolkit, session: &Session) {
        self::discard(kit.journal_dir(), session);
    }
}

/// Replay a session's journal, if it has one, and say what came back. With
/// `recover` false the journal is discarded instead. `None` when there was
/// nothing to recover.
pub fn recover(kit: &mut Toolkit, handle: &str, recover: bool) -> Result<Option<Data>, ToolError> {
    Ok(recovered(kit, handle, recover)?.map(|report| super::data::from_json(&report)))
}

/// [`recover`], its report as the JSON it is built as.
fn recovered(kit: &mut Toolkit, handle: &str, recover: bool) -> Result<Option<Value>, ToolError> {
    let Some(dir) = kit.journal_dir().map(Path::to_path_buf) else { return Ok(None) };
    let session = kit.session(handle)?;
    let Some(base) = session.base else { return Ok(None) };
    let path = path_for(&dir, session);
    let journal = session.ports.journal.clone();
    let Some(text) = journal.read(&path) else { return Ok(None) };

    let mut lines: Vec<&str> = text.lines().filter(|line| !line.trim().is_empty()).collect();
    let header_line = if lines.is_empty() { "" } else { lines.remove(0) };
    let stored: Option<Value> = serde_json::from_str(header_line).ok();
    let recorded = stored.as_ref().and_then(|h| h.get("base")).and_then(base_from_json);
    let version = stored.as_ref().and_then(|h| h.get("grafta_journal")).and_then(Value::as_u64);

    if !recover {
        journal.remove(&path);
        return Ok(Some(json!({ "edits": 0, "discarded": lines.len() })));
    }
    if version != Some(VERSION) || recorded.is_none() {
        let aside = set_aside(journal.as_ref(), &path, "unreadable", &text)?;
        return Ok(Some(json!({
            "edits": 0,
            "set_aside": { "path": aside, "reason": "the journal's header could not be read" },
        })));
    }
    if recorded != Some(base) {
        let was = recorded.expect("checked above");
        let aside = set_aside(journal.as_ref(), &path, "stale", &text)?;
        return Ok(Some(json!({
            "edits": 0,
            "set_aside": {
                "path": aside,
                "entries": lines.len(),
                "reason": format!(
                    "the file changed after these edits were made: they were made on {} bytes \
                     with fingerprint {:016x}, and it is now {} bytes with fingerprint {:016x}",
                    was.bytes, was.fnv, base.bytes, base.fnv
                ),
            },
        })));
    }

    // A last line that does not parse was being written when the server
    // stopped. Its result was never returned, so it is dropped and said.
    let torn = lines.last().is_some_and(|line| serde_json::from_str::<Value>(line).is_err());
    if torn {
        lines.pop();
    }

    kit.set_replaying(true);
    let mut kept: Vec<String> = Vec::new();
    let mut stopped: Option<Stop> = None;
    for line in &lines {
        match replay(kit, handle, line) {
            Ok(()) => kept.push((*line).to_string()),
            Err(stop) => {
                stopped = Some(stop);
                break;
            }
        }
    }
    kit.set_replaying(false);
    let replayed = kept.len();

    let mut out = json!({ "edits": replayed });
    if torn {
        out["dropped"] = json!({
            "entries": 1,
            "reason": "the last edit was being journaled when grafta stopped, and its result was never returned",
        });
    }
    let session = kit.session(handle)?;
    let head = header(session, base).to_string();
    let mut rest_from = replayed;
    if let Some(Stop { applied: Some(entry), reason }) = &stopped {
        // The call ran, and its effect is in the file now, so it stays in the
        // journal as what it did this time. The model is told, because what
        // it did the first time is what the model's later calls assumed.
        kept.push(entry.to_string());
        rest_from += 1;
        out["diverged"] = json!({ "at": replayed, "reason": reason, "kept": true });
    }
    let kept: String = kept.iter().map(|line| format!("{line}\n")).collect();
    journal
        .rewrite(&path, &format!("{head}\n{kept}"))
        .map_err(|error| DomainError::CannotRewriteJournal { path: path.clone(), error: error.to_string() })?;
    if let Some(stop) = stopped {
        if rest_from < lines.len() {
            let rest: String = lines[rest_from..].iter().map(|line| format!("{line}\n")).collect();
            let aside = set_aside(journal.as_ref(), &path, "unreplayed", &format!("{head}\n{rest}"))?;
            out["unreplayed"] = json!({
                "entries": lines.len() - rest_from,
                "path": aside,
                "from": rest_from,
                "reason": if stop.applied.is_some() {
                    "an earlier edit came out differently, so these may no longer mean what they meant".to_string()
                } else {
                    stop.reason
                },
            });
        }
    }
    Ok(Some(out))
}

/// Why a replay stopped. `applied` is the entry as it ran this time, when the
/// call ran but came out differently; `None` when it did not run.
struct Stop {
    applied: Option<Value>,
    reason: String,
}

impl From<String> for Stop {
    fn from(reason: String) -> Self {
        Self { applied: None, reason }
    }
}

/// Replay one entry and compare what it produced with what it produced first.
fn replay(kit: &mut Toolkit, handle: &str, line: &str) -> Result<(), Stop> {
    let entry: Value = serde_json::from_str(line).map_err(|error| format!("the entry does not parse: {error}"))?;
    let tool = entry.get("tool").and_then(Value::as_str).ok_or_else(|| "the entry names no tool".to_string())?;
    let mut args = entry.get("args").cloned().unwrap_or_else(|| json!({}));
    if let Some(fields) = args.as_object_mut() {
        fields.insert("handle".into(), json!(handle));
    }
    let output = crate::call(kit, tool, &args).map_err(|error| format!("{tool} failed: {error}"))?;
    let session = kit.session(handle).map_err(|error| error.to_string())?;
    let now = check(session, &output);
    let then = entry.get("check").cloned().unwrap_or(Value::Null);
    if now != then {
        let mut applied = entry.clone();
        applied["check"] = now.clone();
        return Err(Stop { applied: Some(applied), reason: format!("{tool} produced {now}, not {then} as when it was made") });
    }
    Ok(())
}

/// Write `text` beside the journal as `<journal>.<why>-<time>.jsonl`, and
/// remove the journal when it is the one being set aside.
fn set_aside(port: &dyn Journal, journal: &Path, why: &str, text: &str) -> Result<String, ToolError> {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let stem = journal.file_stem().map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let aside = journal.with_file_name(format!("{stem}.{why}-{seconds}.jsonl"));
    port.write(&aside, text)
        .map_err(|error| DomainError::CannotSetAside { path: aside.clone(), error: error.to_string() })?;
    if why != "unreplayed" {
        port.remove(journal);
    }
    Ok(aside.display().to_string())
}

