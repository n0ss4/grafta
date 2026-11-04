//! Keeping every result small enough for a model, and saying so when it is cut.
//!
//! A result a model cannot tell is partial is worse than a short one: it will
//! treat a truncated tree as the whole tree. So nothing here drops anything
//! silently. A page of a list says how many there are in total and how to get
//! the next page, and a cut subtree says where to resume.

use crate::data::{data, Data};
use crate::port::Measure;
use crate::{DomainError, ToolError};

/// The default result size: about 10,000 tokens, where Claude Code starts to
/// warn (research D4).
pub const DEFAULT_BUDGET: usize = 32 * 1024;
/// The smallest a caller may ask for: room for a page's own fields.
pub const MIN_BUDGET: usize = 1024;
/// The largest a caller may ask for: under Claude Code's default cap of 25,000
/// tokens.
pub const MAX_BUDGET: usize = 96 * 1024;

/// How many bytes of JSON one result may take, as [`Measure`] counts them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget(pub usize);

/// Where a paged list is up to, tied to the state of the session it was issued
/// against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub offset: usize,
    pub edits: u32,
}

impl Cursor {
    #[must_use]
    pub fn encode(self) -> String {
        format!("{}.{}", self.offset, self.edits)
    }

    /// Read a cursor, refusing one issued before the session changed.
    pub fn decode(text: &str, edits: u32) -> Result<Self, ToolError> {
        let parsed = text
            .split_once('.')
            .and_then(|(offset, at)| Some(Self { offset: offset.parse().ok()?, edits: at.parse().ok()? }));
        let cursor = parsed.ok_or_else(|| DomainError::NotACursor { text: text.into() })?;
        if cursor.edits != edits {
            return Err(DomainError::StaleCursor.into());
        }
        Ok(cursor)
    }
}

/// A page asked for: how many results, from where, in how many bytes. The
/// edge reads it; `limit` is the tool's own default when not given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paging {
    pub limit: Option<usize>,
    pub cursor: Option<String>,
    pub budget: Budget,
}

/// One page of a longer list: the items, the total, and the way on.
///
/// `reserve` is what the caller will add to the result afterwards, in bytes,
/// so that the whole result and not only the list stays within the budget.
pub fn page<T>(
    items: Vec<T>,
    paging: &Paging,
    edits: u32,
    default_limit: usize,
    reserve: usize,
    measure: &dyn Measure,
    item: impl Fn(T) -> Data,
) -> Result<Data, ToolError> {
    let limit = paging.limit.unwrap_or(default_limit);
    let offset = match &paging.cursor {
        Some(text) => Cursor::decode(text, edits)?.offset,
        None => 0,
    };
    let total = items.len();
    let budget = paging.budget;

    let mut results = Vec::new();
    let mut used = 256 + reserve;
    let mut next = offset;
    for value in items.into_iter().skip(offset).take(limit).map(item) {
        let size = measure.bytes(&value) + 1;
        if used + size > budget.0 && !results.is_empty() {
            break;
        }
        used += size;
        results.push(value);
        next += 1;
    }

    let mut out = data!({ "total": total, "results": results });
    if next < total {
        out["next_cursor"] = data!(Cursor { offset: next, edits }.encode());
        out["omitted"] = data!({
            "reason": "page",
            "remaining": total - next,
            "how": "call again with this next_cursor",
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
