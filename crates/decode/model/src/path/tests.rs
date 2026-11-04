use super::*;

fn u(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(b[i..i + 4].try_into().unwrap())
}

fn f(b: &[u8], i: usize) -> f32 {
    f32::from_le_bytes(b[i..i + 4].try_into().unwrap())
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.05
}

#[test]
fn lucides_check_is_two_lines_from_implicit_and_relative_commands() {
    let path = Path::parse("M20 6 9 17l-5-5").unwrap();
    assert_eq!(path.subpaths.len(), 1);
    let s = &path.subpaths[0];
    assert_eq!(s.start, (20.0, 6.0));
    assert_eq!(s.segments, [Segment::Line((9.0, 17.0)), Segment::Line((4.0, 12.0))]);
    assert!(!s.closed);
}

#[test]
fn horizontal_vertical_and_close_draw_a_closed_square() {
    let path = Path::parse("m2,2 h10 v10 H2 z").unwrap();
    let s = &path.subpaths[0];
    assert!(s.closed);
    assert_eq!(s.segments.last(), Some(&Segment::Line((2.0, 12.0))));
    assert_eq!(path.bounds(), Some((2.0, 2.0, 12.0, 12.0)));
}

#[test]
fn two_arcs_draw_a_circle_with_its_true_bounds() {
    // Lucide's circle, as path data: centre 12,12, radius 10.
    let path = Path::parse("M12 2a10 10 0 1 0 0 20a10 10 0 1 0 0-20z").unwrap();
    let (a, b, c, d) = path.bounds().unwrap();
    assert!(near(a, 2.0) && near(b, 2.0) && near(c, 22.0) && near(d, 22.0), "{:?}", (a, b, c, d));
    // Every point of every cubic stays on the circle, near enough to draw.
    let s = &path.subpaths[0];
    let mut from = s.start;
    for segment in &s.segments {
        let Segment::Cubic(c1, c2, to) = *segment else { panic!("an arc is cubics") };
        for i in 0..=8 {
            let (x, y) = cubic_at(from, c1, c2, to, i as f32 / 8.0);
            let r = ((x - 12.0).powi(2) + (y - 12.0).powi(2)).sqrt();
            assert!((r - 10.0).abs() < 0.03, "radius {r}");
        }
        from = to;
    }
}

#[test]
fn a_quadratic_becomes_the_cubic_that_draws_it() {
    let path = Path::parse("M0 0Q10 20 20 0").unwrap();
    let Segment::Cubic(c1, c2, to) = path.subpaths[0].segments[0] else { panic!() };
    let (x, y) = cubic_at((0.0, 0.0), c1, c2, to, 0.5);
    assert!(near(x, 10.0) && near(y, 10.0), "{x},{y}");
    assert_eq!(path.bounds().map(|b| near(b.3, 10.0)), Some(true));
}

#[test]
fn smooth_curves_reflect_the_previous_control_point() {
    let path = Path::parse("M0 0C0 10 10 10 10 0S20 -10 20 0").unwrap();
    let Segment::Cubic(c1, _, _) = path.subpaths[0].segments[1] else { panic!() };
    assert_eq!(c1, (10.0, -10.0));
}

#[test]
fn the_commands_blob_reads_back_as_the_same_path() {
    let path = Path::parse("M1 2L3 4C5 6 7 8 9 10Z").unwrap();
    let (d, bounds) = crate::vector::decode_path(&path.commands_blob()).unwrap();
    assert_eq!(d, "M 1 2 L 3 4 C 5 6 7 8 9 10 Z");
    assert_eq!(bounds, (1.0, 2.0, 9.0, 10.0));
}

#[test]
fn a_closed_path_is_a_loop_that_returns_to_its_first_vertex() {
    let blob = Path::parse("M0 0L10 0L10 10Z").unwrap().network_blob(true);
    let (vertices, segments, regions) = (u(&blob, 0), u(&blob, 4), u(&blob, 8));
    assert_eq!((vertices, segments, regions), (3, 3, 1));
    let segment = |i: usize| {
        let at = 12 + 3 * 12 + i * 28;
        (u(&blob, at + 4), u(&blob, at + 16))
    };
    assert_eq!([segment(0), segment(1), segment(2)], [(0, 1), (1, 2), (2, 0)], "Z closes back to vertex 0");
    let region = 12 + 3 * 12 + 3 * 28;
    assert_eq!(u(&blob, region), 1, "style 0, NONZERO");
    assert_eq!(u(&blob, region + 4), 1, "one loop");
    assert_eq!((u(&blob, region + 8), u(&blob, region + 12)), (3, 0));
    assert_eq!(blob.len(), region + 8 + 4 * 4);
    assert_eq!(Path::parse("M0 0L1 0L1 1Z").unwrap().network_blob(false)[region - 3 * 28 + 3 * 28..][0], 0, "ODD");
}

#[test]
fn an_open_path_is_segments_with_no_region_and_curves_keep_their_tangents() {
    let blob = Path::parse("M0 0C0 5 5 10 10 10").unwrap().network_blob(true);
    assert_eq!((u(&blob, 0), u(&blob, 4), u(&blob, 8)), (2, 1, 0));
    let at = 12 + 2 * 12;
    assert_eq!((f(&blob, at + 8), f(&blob, at + 12)), (0.0, 5.0), "start tangent: c1 - start");
    assert_eq!((f(&blob, at + 20), f(&blob, at + 24)), (-5.0, 0.0), "end tangent: c2 - end");
    assert_eq!(blob.len(), at + 28);
}

#[test]
fn path_data_that_cannot_be_drawn_is_refused_with_the_reason() {
    for bad in ["L1 2", "M1", "M0 0 X1 1", "", "M0 0 A1 1 0 2 0 3 3", "M0 0Z 1 1"] {
        assert!(Path::parse(bad).is_err(), "{bad:?} was accepted");
    }
}

#[test]
fn a_network_grafta_writes_decodes_back_to_its_path() {
    let path = Path::parse("M0 0L10 0C10 5 5 10 0 10ZM20 0L30 0").unwrap();
    let network = Network::decode(&path.network_blob(false)).unwrap();
    assert_eq!(network.vertices, [(0.0, 0.0), (10.0, 0.0), (0.0, 10.0), (20.0, 0.0), (30.0, 0.0)]);
    assert_eq!(network.segments.len(), 4);
    assert_eq!(network.regions, [(false, vec![vec![0, 1, 2]])]);
    let (fill, nonzero) = network.fill_path((1.0, 1.0)).unwrap();
    assert!(!nonzero);
    assert_eq!(fill, "M 0 0 L 10 0 C 10 5 5 10 0 10 L 0 0 Z");
    assert_eq!(network.stroke_path((2.0, 1.0)), "M 0 0 L 20 0 C 20 5 10 10 0 10 L 0 0 M 40 0 L 60 0");
}

#[test]
fn a_blob_that_is_not_a_network_is_refused() {
    let blob = Path::parse("M0 0L1 1").unwrap().network_blob(true);
    assert!(Network::decode(&blob[..blob.len() - 1]).is_none(), "truncated");
    let mut longer = blob.clone();
    longer.push(0);
    assert!(Network::decode(&longer).is_none(), "trailing bytes");
    assert!(Network::decode(&[255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0]).is_none(), "impossible counts");
}

#[test]
fn a_subpath_back_at_its_start_is_closed_without_z() {
    // Lucide writes circles as two arcs ending where they began, with no Z.
    let blob = Path::parse("M2 12a10 10 0 1 0 20 0a10 10 0 1 0-20 0").unwrap().network_blob(true);
    let network = Network::decode(&blob).unwrap();
    assert_eq!(network.regions.len(), 1, "fillable");
    let loop_ = &network.regions[0].1[0];
    let (first, last) = (network.segments[loop_[0] as usize], network.segments[*loop_.last().unwrap() as usize]);
    assert_eq!(last.2, first.0, "the last segment returns to the first vertex");
}
