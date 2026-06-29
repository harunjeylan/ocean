use crate::ocean_chunk::heading::detect_heading;

#[test]
fn detects_h1() {
    assert_eq!(detect_heading("# Hello"), Some((1, "Hello".into())));
}

#[test]
fn detects_h2() {
    assert_eq!(detect_heading("## Sub Title"), Some((2, "Sub Title".into())));
}

#[test]
fn detects_h3() {
    assert_eq!(detect_heading("### Deep"), Some((3, "Deep".into())));
}

#[test]
fn detects_h6() {
    assert_eq!(detect_heading("###### Tiny"), Some((6, "Tiny".into())));
}

#[test]
fn returns_none_for_plain_text() {
    assert_eq!(detect_heading("Just a paragraph."), None);
}

#[test]
fn returns_none_for_empty() {
    assert_eq!(detect_heading(""), None);
}

#[test]
fn returns_none_for_just_hash() {
    assert_eq!(detect_heading("# "), None);
}

#[test]
fn handles_leading_whitespace() {
    assert_eq!(detect_heading("  # Indented"), Some((1, "Indented".into())));
}

#[test]
fn handles_trailing_whitespace() {
    assert_eq!(detect_heading("# Title   "), Some((1, "Title".into())));
}

#[test]
fn hash_without_space_is_not_heading() {
    assert_eq!(detect_heading("#NotHeading"), None);
}
