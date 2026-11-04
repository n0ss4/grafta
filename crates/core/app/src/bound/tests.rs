use super::*;

/// A value measured by its own writing, which is JSON's.
struct Written;

impl Measure for Written {
    fn bytes(&self, data: &Data) -> usize {
        data.to_string().len()
    }
}

fn page<T>(items: Vec<T>, paging: &Paging, edits: u32, default_limit: usize, reserve: usize, item: impl Fn(T) -> Data) -> Result<Data, ToolError> {
    super::page(items, paging, edits, default_limit, reserve, &Written, item)
}

fn numbers(n: usize) -> Vec<usize> {
    (0..n).collect()
}

fn asked(limit: Option<usize>, cursor: Option<&str>, budget: usize) -> Paging {
    Paging { limit, cursor: cursor.map(str::to_owned), budget: Budget(budget) }
}

#[test]
fn paging_covers_every_item_exactly_once() {
    let mut seen = Vec::new();
    let mut asking = asked(Some(7), None, DEFAULT_BUDGET);
    loop {
        let out = page(numbers(30), &asking, 0, 50, 0, |n| data!(n)).unwrap();
        assert_eq!(out["total"], 30);
        seen.extend(out["results"].as_list().unwrap().iter().map(|v| v.as_u64().unwrap()));
        match out.get("next_cursor") {
            Some(cursor) => asking = asked(Some(7), cursor.as_str(), DEFAULT_BUDGET),
            None => break,
        }
    }
    assert_eq!(seen, (0..30).collect::<Vec<u64>>());
}

#[test]
fn a_last_page_says_nothing_was_omitted_and_an_early_one_says_what_was() {
    let early = page(numbers(10), &asked(Some(4), None, DEFAULT_BUDGET), 0, 50, 0, |n| data!(n)).unwrap();
    assert_eq!(early["omitted"]["remaining"], 6);

    let whole = page(numbers(3), &asked(None, None, DEFAULT_BUDGET), 0, 50, 0, |n| data!(n)).unwrap();
    assert!(whole.get("omitted").is_none());
    assert!(whole.get("next_cursor").is_none());
}

#[test]
fn an_empty_list_still_states_its_total() {
    let out = page(Vec::<usize>::new(), &asked(None, None, DEFAULT_BUDGET), 0, 50, 0, |n| data!(n)).unwrap();
    assert_eq!(out["total"], 0);
    assert_eq!(out["results"], Data::List(Vec::new()));
}

#[test]
fn a_stale_cursor_is_refused_with_its_reason() {
    let cursor = Cursor { offset: 5, edits: 1 }.encode();
    let error = Cursor::decode(&cursor, 2).unwrap_err();
    assert!(matches!(&error, ToolError::Refused(message) if message.contains("stale")));
}

#[test]
fn a_budget_cut_is_reported_not_silent() {
    let big = "x".repeat(900);
    let out = page(numbers(100), &asked(Some(1000), None, 4096), 0, 1000, 0, |_| {
        data!(big)
    })
    .unwrap();
    let shown = out["results"].as_list().unwrap().len();
    assert!(shown < 100 && shown > 0);
    assert!(out.to_string().len() <= 4096 + 512);
    assert_eq!(out["omitted"]["remaining"], 100 - shown);
}
