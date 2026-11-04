//! Laying out the auto-layout frames grafta created.
//!
//! Figma does not recompute auto-layout when it opens a file: it trusts the
//! positions and sizes stored in it. A ladder of files opened in Figma
//! showed this on 2026-09-26. Frame children written at 0,0 stayed there, and a hugging frame
//! written 600 px tall stayed 600 px. The one exception was a card whose
//! children were text: Figma lays text out itself, and a text that changes
//! size makes its auto-layout parent lay out again.
//!
//! So grafta lays out what it authors, the way Figma would, after every edit:
//! each auto-layout frame it created, deepest first, so that a nested frame's
//! hugged size is known before its parent uses it. A text child's size is
//! grafta's estimate. Figma corrects it on open and relays out that frame, as
//! the ladder's text card showed.

use kiwi::Value as Raw;
use model::{AxisSizing, LayoutMode, SceneNode};
use schema::{object, Field, Guid, StackCounterAlign, StackJustify, StackPositioning, Wire, WireMut, WireName};

use crate::{Session, ToolError};

/// Lay out every auto-layout frame grafta created in this session.
///
/// A child that grows or stretches changes size, which can change the layout
/// of a frame inside it, so the pass repeats until nothing moves (at most
/// four times; each pass only shrinks what is still unsettled).
pub fn settle(session: &mut Session) -> Result<(), ToolError> {
    for _ in 0..4 {
        if !pass(session)? {
            break;
        }
    }
    Ok(())
}

/// One pass. Returns whether anything changed.
fn pass(session: &mut Session) -> Result<bool, ToolError> {
    let mut frames: Vec<(usize, Guid)> = session
        .created
        .iter()
        .chain(&session.laid_out)
        .filter_map(|&id| session.graph.node(id))
        .filter(|node| node.layout.mode != LayoutMode::None)
        .map(|node| (depth(session, node), node.id))
        .collect();
    frames.sort_by_key(|frame| std::cmp::Reverse(frame.0));

    let mut changed = false;
    for (_, id) in frames {
        let writes = lay_out(session, id);
        let mut touched = Vec::new();
        for (node, fields) in writes {
            if write(session, node, fields)? {
                touched.push(node);
            }
        }
        if !touched.is_empty() {
            changed = true;
            session.refresh(&touched);
        }
    }
    Ok(changed)
}

fn depth(session: &Session, node: &SceneNode) -> usize {
    let mut depth = 0;
    let mut current = node.parent_id;
    while let Some(id) = current {
        depth += 1;
        current = session.graph.node(id).and_then(|n| n.parent_id);
    }
    depth
}

/// A size or a position on one axis, with the frame's axes named by role.
struct Axes {
    horizontal: bool,
}

impl Axes {
    fn primary(&self, w: f32, h: f32) -> f32 {
        if self.horizontal { w } else { h }
    }
    fn counter(&self, w: f32, h: f32) -> f32 {
        if self.horizontal { h } else { w }
    }
    /// (width, height) from (primary, counter).
    fn size(&self, primary: f32, counter: f32) -> (f32, f32) {
        if self.horizontal { (primary, counter) } else { (counter, primary) }
    }
}

/// A position and a size, either of which may be left as it is.
type Placement = (Option<(f32, f32)>, Option<(f32, f32)>);

/// What to write, per node.
type Writes = Vec<(Guid, Placement)>;

/// The layout of one frame, as Figma computes it for the properties grafta
/// can set: flow along the primary axis with spacing and padding, MIN, CENTER,
/// MAX and SPACE_BETWEEN, hug and fixed sizing, grow, cross-axis alignment
/// and STRETCH. Hidden and absolutely positioned children are left where they
/// are, as Figma leaves them.
fn lay_out(session: &Session, id: Guid) -> Writes {
    let Some(frame) = session.graph.node(id) else { return Vec::new() };
    let l = &frame.layout;
    let axes = Axes { horizontal: l.mode == LayoutMode::Horizontal };
    let (pad_start, pad_end, pad_cstart, pad_cend) = if axes.horizontal {
        (l.padding_left, l.padding_right, l.padding_top, l.padding_bottom)
    } else {
        (l.padding_top, l.padding_bottom, l.padding_left, l.padding_right)
    };

    let children: Vec<&SceneNode> = session
        .graph
        .children(id)
        .filter(|c| c.visible && c.layout.positioning != StackPositioning::Absolute.as_wire())
        .collect();
    let n = children.len();

    // Counter axis first: a hugging frame is as thick as its thickest child
    // that does not stretch. A stretching child fills the frame.
    let widest = children
        .iter()
        .filter(|c| c.layout.align_self != StackCounterAlign::Stretch.as_wire())
        .map(|c| axes.counter(c.width, c.height))
        .fold(0.0_f32, f32::max);
    let frame_counter = if l.counter_axis_sizing == AxisSizing::Hug {
        pad_cstart + widest + pad_cend
    } else {
        axes.counter(frame.width, frame.height)
    };
    let counter_room = (frame_counter - pad_cstart - pad_cend).max(0.0);

    // Primary axis: growing children share what the fixed ones leave.
    let spacing_total = l.item_spacing * n.saturating_sub(1) as f32;
    let fixed_sum: f32 = children
        .iter()
        .filter(|c| !(c.layout.grow > 0.0 && l.primary_axis_sizing == AxisSizing::Fixed))
        .map(|c| axes.primary(c.width, c.height))
        .sum();
    let growers = children
        .iter()
        .filter(|c| c.layout.grow > 0.0 && l.primary_axis_sizing == AxisSizing::Fixed)
        .count();
    let frame_primary = if l.primary_axis_sizing == AxisSizing::Fixed {
        axes.primary(frame.width, frame.height)
    } else {
        pad_start + fixed_sum + spacing_total + pad_end
    };
    let room = frame_primary - pad_start - pad_end;
    let grown = if growers > 0 { ((room - fixed_sum - spacing_total) / growers as f32).max(0.0) } else { 0.0 };

    // Each child within its own minimum and maximum. A grower
    // held by its maximum does not give its share to the others: Figma does,
    // grafta does not yet.
    let clamp = |value: f32, min: Option<f32>, max: Option<f32>| {
        let value = max.map_or(value, |m| value.min(m));
        min.map_or(value, |m| value.max(m))
    };
    let sizes: Vec<(f32, f32)> = children
        .iter()
        .map(|c| {
            let primary = if c.layout.grow > 0.0 && l.primary_axis_sizing == AxisSizing::Fixed {
                grown
            } else {
                axes.primary(c.width, c.height)
            };
            let counter = if c.layout.align_self == StackCounterAlign::Stretch.as_wire() { counter_room } else { axes.counter(c.width, c.height) };
            let (min_p, max_p, min_c, max_c) = if axes.horizontal {
                (c.min_width, c.max_width, c.min_height, c.max_height)
            } else {
                (c.min_height, c.max_height, c.min_width, c.max_width)
            };
            (clamp(primary, min_p, max_p), clamp(counter, min_c, max_c))
        })
        .collect();
    let content: f32 = sizes.iter().map(|s| s.0).sum::<f32>() + spacing_total;

    let (mut cursor, gap) = match StackJustify::from_wire(&l.primary_axis_align) {
        Some(StackJustify::Center) => (pad_start + (room - content) / 2.0, l.item_spacing),
        Some(StackJustify::Max) => (pad_start + room - content, l.item_spacing),
        Some(StackJustify::SpaceBetween) if n > 1 => {
            let sum: f32 = sizes.iter().map(|s| s.0).sum();
            (pad_start, (room - sum) / (n - 1) as f32)
        }
        _ => (pad_start, l.item_spacing),
    };

    let mut writes: Writes = Vec::new();
    for (child, (primary, counter)) in children.iter().zip(&sizes) {
        let cross = if child.layout.align_self == StackCounterAlign::Stretch.as_wire() {
            pad_cstart
        } else {
            match StackCounterAlign::from_wire(&l.counter_axis_align) {
                Some(StackCounterAlign::Center) => pad_cstart + (counter_room - counter) / 2.0,
                Some(StackCounterAlign::Max) => pad_cstart + counter_room - counter,
                _ => pad_cstart,
            }
        };
        let position = if axes.horizontal { (cursor, cross) } else { (cross, cursor) };
        let size = axes.size(*primary, *counter);
        let resized = (size.0 - child.width).abs() > 0.001 || (size.1 - child.height).abs() > 0.001;
        writes.push((child.id, (Some(position), resized.then_some(size))));
        cursor += primary + gap;
    }

    // A hugging frame within its own minimum and maximum too.
    let (min_p, max_p, min_c, max_c) = if axes.horizontal {
        (frame.min_width, frame.max_width, frame.min_height, frame.max_height)
    } else {
        (frame.min_height, frame.max_height, frame.min_width, frame.max_width)
    };
    let frame_primary = if l.primary_axis_sizing == AxisSizing::Fixed { frame_primary } else { clamp(frame_primary, min_p, max_p) };
    let frame_counter = if l.counter_axis_sizing == AxisSizing::Hug { clamp(frame_counter, min_c, max_c) } else { frame_counter };
    let frame_size = axes.size(frame_primary, frame_counter);
    if (frame_size.0 - frame.width).abs() > 0.001 || (frame_size.1 - frame.height).abs() > 0.001 {
        writes.push((id, (None, Some(frame_size))));
    }
    writes
}

/// Write a position and a size into a node change. Returns whether anything
/// actually changed.
fn write(session: &mut Session, id: Guid, (position, size): Placement) -> Result<bool, ToolError> {
    let Some(fields) = session.fields_mut(id) else { return Ok(false) };
    let mut changed = false;
    if let Some((x, y)) = position {
        if let Some(Raw::Object(t)) = fields.field_mut(Field::Transform) {
            for (key, value) in [(Field::M02, x), (Field::M12, y)] {
                let old = t.field(key).and_then(Raw::as_f32).unwrap_or(0.0);
                if (old - value).abs() > 0.001 {
                    t.set(key, Raw::Float(value));
                    changed = true;
                }
            }
        }
    }
    if let Some((w, h)) = size {
        fields.set(Field::Size,
            object([(Field::X, Raw::Float(w)), (Field::Y, Raw::Float(h))]),
        );
        changed = true;
    }
    Ok(changed)
}

