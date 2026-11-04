//! Figma's vector geometry, decoded into SVG path data.
//!
//! A `.fig` stores each path as a blob of `opcode, f32 args…`, little-endian and
//! back to back. Figma flattens the vector network before writing it, and
//! `strokeGeometry` holds the stroke already outlined into a fillable region —
//! so both kinds of path are painted with `fill`, and neither needs a stroke
//! width reconstructing.

use crate::types::trim_float;

/// Axis-aligned bounds of some path data, in the layer's own coordinates:
/// `(min_x, min_y, max_x, max_y)`.
pub type Bounds = (f32, f32, f32, f32);

const CLOSE: u8 = 0;
const MOVE_TO: u8 = 1;
const LINE_TO: u8 = 2;
const QUAD_TO: u8 = 3;
const CUBIC_TO: u8 = 4;

/// Decode one command blob into SVG path data and the bounds it covers.
///
/// Returns `None` for a blob that does not decode cleanly, so geometry stored in
/// a form this does not understand falls back to a plain box rather than drawing
/// something wrong.
#[must_use]
pub fn decode_path(blob: &[u8]) -> Option<(String, Bounds)> {
    let mut d = String::new();
    let (mut min_x, mut min_y) = (f32::MAX, f32::MAX);
    let (mut max_x, mut max_y) = (f32::MIN, f32::MIN);
    let mut at = 0usize;

    while at < blob.len() {
        let (letter, args) = match blob[at] {
            CLOSE => ("Z", 0usize),
            MOVE_TO => ("M", 2),
            LINE_TO => ("L", 2),
            QUAD_TO => ("Q", 4),
            CUBIC_TO => ("C", 6),
            _ => return None,
        };
        at += 1;
        if at + args * 4 > blob.len() {
            return None;
        }

        if !d.is_empty() {
            d.push(' ');
        }
        d.push_str(letter);
        for arg in 0..args {
            let start = at + arg * 4;
            let value =
                f32::from_le_bytes([blob[start], blob[start + 1], blob[start + 2], blob[start + 3]]);
            if !value.is_finite() {
                return None;
            }
            if arg % 2 == 0 {
                min_x = min_x.min(value);
                max_x = max_x.max(value);
            } else {
                min_y = min_y.min(value);
                max_y = max_y.max(value);
            }
            d.push(' ');
            d.push_str(&trim_float(value));
        }
        at += args * 4;
    }

    (min_x <= max_x).then_some((d, (min_x, min_y, max_x, max_y)))
}

/// Grow `bounds` to cover `other`.
pub fn extend(bounds: &mut Option<Bounds>, other: Bounds) {
    *bounds = Some(match *bounds {
        None => other,
        Some((min_x, min_y, max_x, max_y)) => (
            min_x.min(other.0),
            min_y.min(other.1),
            max_x.max(other.2),
            max_y.max(other.3),
        ),
    });
}

#[cfg(test)]
mod tests;
