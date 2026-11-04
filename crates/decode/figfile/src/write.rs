//! Putting a [`FigFile`] back into the bytes of a `.fig`.
//!
//! The mirror of the read path, and it never authors anything. The schema is
//! the one that came out of the file (research D1), the message is the decoded
//! value re-encoded against that schema, and every region grafta does not
//! interpret is copied through untouched (I2). Nothing is normalised, repaired
//! or tidied on the way past — a round trip that improves the file is not a
//! round trip.

use crate::{archive, compress, container, FigError, FigFile};

/// Write a `FigFile` as the bytes of a complete `.fig` archive.
///
/// Deterministic: the same input produces the same bytes every time (I4).
pub fn write_fig(fig: &FigFile) -> Result<Vec<u8>, FigError> {
    let canvas = write_canvas(fig)?;
    archive::write(&fig.canvas_name, &canvas, &fig.other_entries, &fig.images)
}

/// Write the container layer alone, mirroring [`crate::parse_canvas`] — for
/// callers whose canvas is not going into a ZIP.
pub fn write_canvas(fig: &FigFile) -> Result<Vec<u8>, FigError> {
    // The schema goes back exactly as Figma shipped it — D1. It is carried,
    // never authored, so this chunk is a copy and not a re-encoding.
    let schema_chunk = fig.schema_chunk.as_slice();

    let message_bytes = kiwi::encode_message("Message", &fig.node_changes, &fig.schema)?;
    let message_chunk =
        compress::deflate(&message_bytes).map_err(|e| FigError::Deflate(e.to_string()))?;

    let mut chunks: Vec<&[u8]> = vec![schema_chunk, &message_chunk];
    chunks.extend(fig.trailing_chunks.iter().map(Vec::as_slice));

    Ok(container::write(fig.product, fig.version, &chunks))
}

// ── Comparison ───────────────────────────────────────────────────────────────

/// Where two documents first differ, and how.
///
/// A boolean is not enough: a failure has to be a place a reader can
/// go to without searching, because investigating this format is the expensive
/// thing this whole feature exists to avoid doing twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    /// Where it is — `nodeChanges[41].name`, `images[c390b1…]`, `version`.
    pub path: String,
    /// What the original held.
    pub left: String,
    /// What the round trip produced.
    pub right: String,
}

impl std::fmt::Display for Difference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} became {}", self.path, self.left, self.right)
    }
}

/// The first difference between two parsed files, or `None` when they agree.
///
/// Compares what grafta read: the version, the message, the images, and the
/// preserved regions. It does **not** compare raw bytes — deflate is not
/// canonical and two correct writers disagree on them (research D6).
#[must_use]
pub fn compare(left: &FigFile, right: &FigFile) -> Option<Difference> {
    if left.version != right.version {
        return Some(Difference {
            path: "version".into(),
            left: left.version.to_string(),
            right: right.version.to_string(),
        });
    }
    if left.canvas_name != right.canvas_name {
        return Some(Difference {
            path: "canvas".into(),
            left: left.canvas_name.clone(),
            right: right.canvas_name.clone(),
        });
    }

    compare_value("nodeChanges", &left.node_changes, &right.node_changes)
        .or_else(|| compare_entries("images", &left.images, &right.images))
        .or_else(|| compare_entries("entries", &left.other_entries, &right.other_entries))
        .or_else(|| compare_chunks(&left.trailing_chunks, &right.trailing_chunks))
}

fn compare_entries(
    what: &str,
    left: &[(String, Vec<u8>)],
    right: &[(String, Vec<u8>)],
) -> Option<Difference> {
    if left.len() != right.len() {
        return Some(Difference {
            path: what.to_string(),
            left: format!("{} of them", left.len()),
            right: format!("{} of them", right.len()),
        });
    }
    for (index, (one, two)) in left.iter().zip(right).enumerate() {
        if one.0 != two.0 {
            return Some(Difference {
                path: format!("{what}[{index}]"),
                left: one.0.clone(),
                right: two.0.clone(),
            });
        }
        if one.1 != two.1 {
            return Some(Difference {
                path: format!("{what}[{}]", one.0),
                left: format!("{} bytes", one.1.len()),
                right: format!("{} bytes", two.1.len()),
            });
        }
    }
    None
}

fn compare_chunks(left: &[Vec<u8>], right: &[Vec<u8>]) -> Option<Difference> {
    if left.len() != right.len() {
        return Some(Difference {
            path: "chunks".into(),
            left: format!("{} of them", left.len()),
            right: format!("{} of them", right.len()),
        });
    }
    left.iter().zip(right).enumerate().find(|(_, (one, two))| one != two).map(|(index, (one, two))| {
        Difference {
            path: format!("chunks[{}]", index + 2),
            left: format!("{} bytes", one.len()),
            right: format!("{} bytes", two.len()),
        }
    })
}

/// Walks two decoded values together, naming the path to the first difference.
#[expect(clippy::disallowed_methods, reason = "compares whatever fields the two files hold, by the names they hold them under")]
fn compare_value(path: &str, left: &kiwi::Value, right: &kiwi::Value) -> Option<Difference> {
    use kiwi::Value;

    let differs = |left: String, right: String| {
        Some(Difference { path: path.to_string(), left, right })
    };

    match (left, right) {
        (Value::Object(one), Value::Object(two)) => {
            for (key, value) in one {
                let Some(other) = two.get(key) else {
                    return differs(format!("has {key}"), "does not".into());
                };
                if let Some(found) = compare_value(&format!("{path}.{key}"), value, other) {
                    return Some(found);
                }
            }
            // Order is data here (D4), so a reordering is a difference even
            // though the two maps hold the same pairs.
            match two.keys().find(|key| !one.contains_key(key)) {
                Some(key) => differs("does not".into(), format!("has {key}")),
                None if one.keys().ne(two.keys()) => {
                    differs(format!("fields in the order {:?}", one.keys().collect::<Vec<_>>()),
                            format!("{:?}", two.keys().collect::<Vec<_>>()))
                }
                None => None,
            }
        }
        (Value::Array(one), Value::Array(two)) => {
            if one.len() != two.len() {
                return differs(format!("{} items", one.len()), format!("{} items", two.len()));
            }
            one.iter().zip(two).enumerate().find_map(|(index, (value, other))| {
                compare_value(&format!("{path}[{index}]"), value, other)
            })
        }
        // NaN is not equal to itself, and a file full of geometry may hold one.
        // Comparing the bits asks the question that matters: is it the same
        // float that came in?
        (Value::Float(one), Value::Float(two)) if one.to_bits() != two.to_bits() => {
            differs(one.to_string(), two.to_string())
        }
        // Equal bits, so the same float — including two NaNs, which `==`
        // would have called different.
        (Value::Float(_), Value::Float(_)) => None,
        (Value::Bytes(one), Value::Bytes(two)) if one != two => {
            differs(format!("{} bytes", one.len()), format!("{} bytes", two.len()))
        }
        _ if left != right => differs(format!("{left:?}"), format!("{right:?}")),
        _ => None,
    }
}
