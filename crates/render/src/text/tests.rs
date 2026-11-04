use super::*;

/// Every character 10 px wide.
fn tens(text: &str) -> Vec<f32> {
    text.chars().map(|_| 10.0).collect()
}

fn lines(text: &str, width: Option<f32>) -> Vec<(String, f32)> {
    let chars: Vec<char> = text.chars().collect();
    break_lines(text, &tens(text), width).into_iter().map(|l| (chars[l.start..l.end].iter().collect(), l.width)).collect()
}

#[test]
fn a_line_breaks_after_the_last_space_that_fits_and_trailing_spaces_are_free() {
    assert_eq!(lines("ab cd ef", Some(55.0)), [("ab cd ".to_string(), 50.0), ("ef".to_string(), 20.0)]);
    assert_eq!(lines("ab cd", Some(20.0)), [("ab ".to_string(), 20.0), ("cd".to_string(), 20.0)]);
}

#[test]
fn a_word_too_long_for_a_line_breaks_where_it_runs_out() {
    assert_eq!(lines("abcdef", Some(35.0)), [("abc".to_string(), 30.0), ("def".to_string(), 30.0)]);
    assert_eq!(lines("abc", Some(5.0)).len(), 3, "one character a line at least");
}

#[test]
fn newlines_start_paragraphs_and_no_width_means_no_wrapping() {
    assert_eq!(lines("ab cd\nef", None), [("ab cd".to_string(), 50.0), ("ef".to_string(), 20.0)]);
    assert_eq!(lines("a\n\nb", Some(100.0)).len(), 3, "an empty paragraph is a line");
}

#[test]
fn a_machine_with_none_of_the_stand_ins_still_finds_a_font() {
    // One installed face that is no stand-in, alone in a database: as a Linux
    // machine with only DejaVu or Liberation is.
    let system = fonts();
    let Some(other) = system.faces().find(|face| {
        face.families.iter().all(|(name, _)| !STAND_INS.contains(&name.as_str()))
            && matches!(face.source, fontdb::Source::File(_))
    }) else {
        return; // No such font here to build that machine from.
    };
    let fontdb::Source::File(path) = &other.source else { unreachable!() };
    let mut db = Database::new();
    db.load_font_file(path).unwrap();
    let font = find(&db, "No Such Family", 400, false).expect("any installed font");
    assert!(font.substituted);
}

#[test]
fn a_style_is_a_face_spaces_and_case_aside() {
    let faces = vec!["Regular".to_string(), "SemiBold".to_string(), "Bold Oblique".to_string()];
    assert!(is_face(&faces, "Semi Bold"));
    assert!(is_face(&faces, "semi-bold"));
    assert!(is_face(&faces, "Bold Oblique"));
    assert!(!is_face(&faces, "Bold Italic"), "Italic and Oblique are different names");
    assert!(!is_face(&faces, "Semibold Extra Wide"));
}

#[test]
fn an_installed_familys_faces_are_named_and_another_family_is_none() {
    let system = fonts();
    let Some(face) = system.faces().find(|face| matches!(face.source, fontdb::Source::File(_))) else {
        return; // No font here to read.
    };
    let fontdb::Source::File(path) = &face.source else { unreachable!() };
    let mut db = Database::new();
    db.load_font_file(path).unwrap();
    let family = db.faces().next().unwrap().families[0].0.clone();
    let names = faces(&db, &family).expect("its family is installed");
    assert!(!names.is_empty(), "{family} has named faces");
    assert_eq!(faces(&db, "No Such Family"), None);
}
