//! A command as one unit of work: the command, then the steps that
//! follow it, each session put back whole if any of them fails.

use super::Command;
use crate::{Handle, Output, ToolError, Toolkit};

/// A command done: its result, the sessions it edited (for the journal), and
/// how many instances' caches were dropped after it (for the result to say).
pub struct Done {
    pub output: Output,
    pub edited: Vec<Handle>,
    pub refreshed: usize,
}

/// Run `command`, named `name` for the rollback check's message.
pub fn run(kit: &mut Toolkit, name: &str, command: &dyn Command) -> Result<Done, ToolError> {
    let mut befores: Vec<(Handle, crate::Before)> = kit.sessions_mut().map(|s| (s.handle.clone(), s.begin())).collect();
    let wholes: Vec<(Handle, Whole)> =
        if kit.checks_rollbacks() { kit.sessions().map(|s| (s.handle.clone(), Whole::of(s))).collect() } else { Vec::new() };
    match steps(kit, command, &befores) {
        Ok(done) => Ok(done),
        Err(error) => {
            for session in kit.sessions_mut() {
                if let Some(at) = befores.iter().position(|(h, _)| *h == session.handle) {
                    session.roll_back(befores.swap_remove(at).1)?;
                }
                if let Some((_, whole)) = wholes.iter().find(|(h, _)| *h == session.handle) {
                    assert!(whole.is(session), "{name} failed ({error}), and its rollback left {} other than before", session.handle);
                }
            }
            Err(error)
        }
    }
}

/// A whole copy of a session's file, for [`Toolkit::check_rollbacks`].
struct Whole {
    node_changes: kiwi::Value,
    blobs: Vec<Vec<u8>>,
    images: Vec<(String, Vec<u8>)>,
}

impl Whole {
    fn of(session: &crate::Session) -> Self {
        Self { node_changes: session.fig.node_changes.clone(), blobs: session.fig.blobs.clone(), images: session.fig.images.clone() }
    }

    fn is(&self, session: &crate::Session) -> bool {
        same(&self.node_changes, &session.fig.node_changes) && self.blobs == session.fig.blobs && self.images == session.fig.images
    }
}

/// Equal, a float compared by its bits: the files carry NaN sizes, and NaN
/// is equal to nothing, itself included.
fn same(left: &kiwi::Value, right: &kiwi::Value) -> bool {
    match (left, right) {
        (kiwi::Value::Float(l), kiwi::Value::Float(r)) => l.to_bits() == r.to_bits(),
        (kiwi::Value::Array(l), kiwi::Value::Array(r)) => l.len() == r.len() && l.iter().zip(r).all(|(l, r)| same(l, r)),
        (kiwi::Value::Object(l), kiwi::Value::Object(r)) => {
            l.len() == r.len() && l.iter().zip(r.iter()).all(|((lk, lv), (rk, rv))| lk == rk && same(lv, rv))
        }
        _ => left == right,
    }
}

/// The command, then auto-layout on each session it edited, then the dropped
/// caches of instances whose components changed.
fn steps(kit: &mut Toolkit, command: &dyn Command, befores: &[(Handle, crate::Before)]) -> Result<Done, ToolError> {
    let output = command.run(kit)?;
    // Figma does not lay auto-layout out on open, so every edit is followed by
    // grafta's own pass over the frames it created.
    let mut edited_handles = Vec::new();
    for session in kit.sessions_mut() {
        let edited = befores.iter().find(|(h, _)| *h == session.handle).is_none_or(|(_, before)| before.edits() != session.edits);
        if edited && session.edits > 0 {
            crate::layout::settle(session)?;
            edited_handles.push(session.handle.clone());
        }
    }
    // A change inside a component makes its instances' caches stale; Figma
    // derives them again when they are gone.
    let mut refreshed = 0;
    for session in kit.sessions_mut() {
        refreshed += crate::tools::guard::drop_caches(session);
    }
    Ok(Done { output, edited: edited_handles, refreshed })
}
