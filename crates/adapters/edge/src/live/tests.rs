use super::*;

#[test]
fn base64_reads_back_what_the_standard_alphabet_writes() {
    for (text, bytes) in [("", &b""[..]), ("Zg==", b"f"), ("Zm8=", b"fo"), ("Zm9v", b"foo"), ("Zm9vYmFy", b"foobar"), ("/+8=", &[0xff, 0xef][..])] {
        assert_eq!(base64(text).as_deref(), Some(bytes), "{text}");
    }
    assert_eq!(base64("Zm9v!"), None);
    assert_eq!(base64("Z"), None);
}

#[test]
fn a_picture_from_figma_becomes_an_image_with_its_caption() {
    let answer = crate::data::from_json(&serde_json::json!({ "image": { "mime": "image/png", "base64": "Zm9v" }, "caption": { "drawn_by": "Figma" } }));
    match picture(answer).unwrap() {
        Output::Image { mime, bytes, caption } => {
            assert_eq!((mime, bytes.as_slice()), ("image/png", &b"foo"[..]));
            assert_eq!(caption.and_then(|c| c.get("drawn_by").and_then(Data::as_str).map(str::to_owned)).as_deref(), Some("Figma"));
        }
        other => panic!("{other:?}"),
    }
    let svg = crate::data::from_json(&serde_json::json!({ "image": { "mime": "image/svg+xml", "base64": "Zm9v" } }));
    assert!(matches!(picture(svg), Err(LiveError::Refused(_))));
}
