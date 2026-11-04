//! Keys between real siblings. Skips without fixtures.

// A test spells the wire apart from `schema`, so a misspelt name fails it.
#![allow(clippy::disallowed_methods)]

use std::collections::HashMap;

use model::author::{node_changes, position_between};

#[test]
fn a_key_fits_between_every_pair_of_adjacent_siblings_in_the_fixtures() {
    for path in testkit::fixture::all() {
        let Some(fig) = testkit::fixture::parsed(path.file_name().unwrap().to_str().unwrap()) else { continue };
        let mut siblings: HashMap<String, Vec<String>> = HashMap::new();
        for nc in node_changes(&fig) {
            let Some(index) = nc.get("parentIndex") else { continue };
            let (Some(parent), Some(position)) = (index.get("guid"), index.get("position").and_then(|p| p.as_str())) else { continue };
            siblings.entry(format!("{parent:?}")).or_default().push(position.to_string());
        }
        let (mut pairs, mut refused) = (0, 0);
        for keys in siblings.values_mut() {
            keys.sort();
            keys.dedup();
            let mut bounds: Vec<(Option<&str>, Option<&str>)> = keys.windows(2).map(|w| (Some(w[0].as_str()), Some(w[1].as_str()))).collect();
            bounds.push((None, keys.first().map(String::as_str)));
            bounds.push((keys.last().map(String::as_str), None));
            for (lower, upper) in bounds {
                pairs += 1;
                match position_between(lower, upper) {
                    Some(key) => {
                        assert!(lower.is_none_or(|l| l < key.as_str()) && upper.is_none_or(|u| key.as_str() < u), "{key:?} not between {lower:?} and {upper:?}");
                    }
                    None => {
                        // Only the one pair no key fits: a key and itself plus spaces.
                        let (l, u) = (lower.unwrap_or(""), upper.expect("a key always fits after the last"));
                        assert!(u.starts_with(l) && u[l.len()..].bytes().all(|b| b == b' '), "no key found between {l:?} and {u:?}");
                        refused += 1;
                    }
                }
            }
        }
        println!("{}: {pairs} gaps between siblings, {refused} with no room", path.display());
    }
}
