//! Every vector network in the fixtures decodes. Skips
//! without fixtures.

// A test spells the wire apart from `schema`, so a misspelt name fails it.
#![allow(clippy::disallowed_methods)]

use model::author::node_changes;
use model::path::Network;

#[test]
fn every_vector_network_in_the_fixtures_decodes_to_paths() {
    for path in testkit::fixture::all() {
        let Some(fig) = testkit::fixture::parsed(path.file_name().unwrap().to_str().unwrap()) else { continue };
        let (mut decoded, mut filled) = (0, 0);
        for nc in node_changes(&fig) {
            let Some(blob) = nc.get("vectorData").and_then(|v| v.get("vectorNetworkBlob")).and_then(|b| b.as_u32()) else { continue };
            let network = Network::decode(&fig.blobs[blob as usize]).unwrap_or_else(|| panic!("{}: blob {blob} does not decode", path.display()));
            let _ = network.stroke_path((1.0, 1.0));
            filled += usize::from(network.fill_path((1.0, 1.0)).is_some());
            decoded += 1;
        }
        println!("{}: {decoded} networks decoded, {filled} with regions", path.display());
    }
}
