use super::*;

/// `moveTo(20, 10)`, `cubicTo(…)`, `close` — the first quadrant of a circle,
/// in the byte layout Figma writes.
fn blob() -> Vec<u8> {
    let mut out = vec![MOVE_TO];
    for value in [20.0f32, 10.0] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.push(CUBIC_TO);
    for value in [20.0f32, 15.5, 15.5, 20.0, 10.0, 20.0] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.push(CLOSE);
    out
}

#[test]
fn decodes_commands_into_svg_path_data() {
    let (d, bounds) = decode_path(&blob()).expect("decodes");
    assert_eq!(d, "M 20 10 C 20 15.5 15.5 20 10 20 Z");
    assert_eq!(bounds, (10.0, 10.0, 20.0, 20.0));
}

#[test]
fn rejects_blobs_it_cannot_read() {
    // An opcode with no meaning, and a command whose arguments run off the end.
    assert!(decode_path(&[200]).is_none());
    assert!(decode_path(&[MOVE_TO, 0, 0]).is_none());
}

#[test]
fn unions_bounds() {
    let mut bounds = None;
    extend(&mut bounds, (0.0, 0.0, 4.0, 4.0));
    extend(&mut bounds, (-2.0, 1.0, 3.0, 9.0));
    assert_eq!(bounds, Some((-2.0, 0.0, 4.0, 9.0)));
}
