use super::*;

#[test]
fn a_site_page_refuses_what_is_put_in_it_and_says_where_to_put_it() {
    let page = Guid::new(1, 44);
    let refused = Page.admits(page, Placement::Frame).expect("a page answers").unwrap_err();
    let ToolError::Refused(words) = refused else { panic!("{refused:?}") };
    assert!(words.contains("1:44") && words.contains("inside a breakpoint"), "{words}");
    assert!(Page.admits(page, Placement::Duplicate).expect("a page answers").is_err(), "a breakpoint copied is a breakpoint added");
}

#[test]
fn only_a_primary_breakpoint_says_so() {
    let node = SceneNode::new(Guid::new(1, 45), "Desktop", NodeType::Frame);
    let primary = schema::object([(Field::IsPrimaryBreakpoint, Raw::Bool(true))]);
    let other = schema::object([(Field::IsPrimaryBreakpoint, Raw::Bool(false))]);
    let json = NoJson;
    assert_eq!(Breakpoint.shows(&node, &primary, &json).and_then(|m| m.get("isPrimaryBreakpoint").cloned()), Some(data!(true)));
    assert!(Breakpoint.shows(&node, &other, &json).is_none());
}

/// No breakpoint reads JSON.
struct NoJson;

impl JsonText for NoJson {
    fn parse(&self, _text: &str) -> Result<Data, String> {
        unreachable!("a breakpoint reads no JSON")
    }
    fn write(&self, _data: &Data) -> String {
        unreachable!("a breakpoint writes no JSON")
    }
}
