//! Every name grafta lists against the schemas of Figma's files, the fixtures.
//! `src/enums.rs`, the format's enums grafta uses with every value those
//! schemas list, is written from them with `GRAFTA_WRITE_ENUMS=1`. With no
//! fixtures, each check skips.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use schema::{Field, WireName};

const WRITE: &str = "GRAFTA_WRITE_ENUMS";
const TARGET: &str = "src/enums.rs";
const FIXTURES: &str = "GRAFTA_FIXTURES";
const DEFAULT_FIXTURES: &str = "../../../fixtures";

/// The schema's enums grafta reads or writes.
const USED: [&str; 50] = [
    "AnimationPhase", "AnimationType", "BlendMode", "ComponentPropNodeField", "ComponentPropType",
    "ConnectionType", "ConnectorLineStyle", "ConnectorMagnet", "ConstraintType", "EasingType", "EffectType",
    "ImageScaleMode", "InteractionType", "LineType", "NavigationType", "NodePhase", "NodeType", "MaskType", "NumberUnits",
    "OverlayBackgroundInteraction", "OverlayBackgroundType", "OverlayPositionType", "PaintType", "PrototypeDeviceType", "ScrollBehavior", "ScrollDirection", "ShapeWithTextType", "SourceDirectionality", "StackAlign", "StackCounterAlign",
    "StackJustify", "StackMode", "StackPositioning", "StackSize", "StackWrap", "StrokeAlign", "StrokeCap",
    "StrokeJoin", "StyleType", "TextAlignHorizontal", "TextAlignVertical", "TextAutoResize", "TextCase",
    "TextDecoration", "TransitionType", "TriggerDevice", "VariableDataType", "VariableField", "VariableResolvedDataType",
    "WindingRule",
];

fn variant(value: &str) -> String {
    let name: String = value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map(|first| first.to_ascii_uppercase().to_string() + &chars.as_str().to_ascii_lowercase()).unwrap_or_default()
        })
        .collect();
    if name.starts_with(|c: char| c.is_ascii_digit()) { format!("V{name}") } else { name }
}

/// The fixtures, read with `figfile` alone: through `testkit` this would need
/// `model`, which may not build until the enums it uses are written.
fn fixtures() -> Vec<figfile::FigFile> {
    let dir = std::env::var_os(FIXTURES).map_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_FIXTURES), Into::into);
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    entries
        .flatten()
        .filter_map(|entry| std::fs::read(entry.path()).ok())
        .filter_map(|bytes| figfile::parse_fig(&bytes).ok())
        .collect()
}

fn values_in_fixtures() -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for fig in fixtures() {
        for definition in fig.schema.definitions.iter().filter(|d| d.kind == kiwi::DefinitionKind::Enum && USED.contains(&d.name.as_str())) {
            out.entry(definition.name.clone()).or_default().extend(definition.fields.iter().map(|f| f.name.clone()));
        }
    }
    out
}

fn source(enums: &BTreeMap<String, BTreeSet<String>>) -> String {
    let mut text = String::from(
        "//! The format's enums grafta uses, with every value the schemas of Figma's\n\
         //! files list. Written by `tests/schemas.rs` from the fixtures; do not edit.\n\n",
    );
    for (name, values) in enums {
        let mut seen = BTreeSet::new();
        text += &format!("crate::wire! {{\n    pub enum {name} {{\n");
        for value in values {
            let v = variant(value);
            assert!(seen.insert(v.clone()), "{name}: two values are named {v}");
            text += &format!("        {v} = \"{value}\",\n");
        }
        text += "    }\n}\n\n";
    }
    let names: Vec<&str> = enums.keys().map(String::as_str).collect();
    text += &format!("crate::wire::wire_value!(\n    {},\n);\n", names.join(",\n    "));
    text
}

#[test]
fn the_enums_say_what_the_schemas_say() {
    let enums = values_in_fixtures();
    if enums.is_empty() {
        return;
    }
    let missing: Vec<&&str> = USED.iter().filter(|name| !enums.contains_key(**name)).collect();
    assert!(missing.is_empty(), "in no fixture's schema: {missing:?}");
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join(TARGET);
    let expected = source(&enums);
    if std::env::var_os(WRITE).is_some() {
        std::fs::write(&target, expected).expect("enums written");
        return;
    }
    let actual = std::fs::read_to_string(&target).unwrap_or_default();
    assert!(actual == expected, "{TARGET} is not what the fixtures' schemas say; write it again with {WRITE}=1");
}

/// Files carry different schemas (a design file has none of FigJam's fields),
/// so a name need only be in one of them.
#[test]
fn every_field_is_in_a_fixtures_schema() {
    let known: BTreeSet<String> =
        fixtures().iter().flat_map(|fig| fig.schema.definitions.iter().flat_map(|d| d.fields.iter().map(|f| f.name.clone()))).collect();
    if known.is_empty() {
        return;
    }
    let missing: Vec<&str> = Field::ALL.iter().map(|f| f.as_wire()).filter(|w| !known.contains(*w)).collect();
    assert!(missing.is_empty(), "in no fixture's schema: {}", missing.join(", "));
}
