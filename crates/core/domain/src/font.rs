//! How a font's faces are named.

/// Whether a style names one of the faces: spaces, hyphens, underscores and
/// case aside, as Figma's `Semi Bold` is a font file's `SemiBold`. Italic and
/// Oblique stay different names, as Figma keeps them.
#[must_use]
pub fn is_face(faces: &[String], style: &str) -> bool {
    let plain = |s: &str| s.chars().filter(|c| !matches!(c, ' ' | '-' | '_')).collect::<String>().to_lowercase();
    faces.iter().any(|face| plain(face) == plain(style))
}
