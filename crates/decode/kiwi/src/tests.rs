use super::*;

/// Manually encode a tiny binary schema for a STRUCT with two fields:
///   struct Point { x: int; y: int; }
/// Binary format (from binary.ts):
///   var_uint(def_count=1)
///   string("Point\0")
///   byte(kind=1 -> STRUCT)
///   var_uint(field_count=2)
///   --- field 0: x
///   string("x\0")
///   var_int(-3)  -> type "int" (PRIMITIVES[2], ~(-3)=2)
///   byte(0)      -> not array
///   var_uint(0)  -> value=0 (unused for STRUCT)
///   --- field 1: y
///   string("y\0")
///   var_int(-3)  -> type "int"
///   byte(0)
///   var_uint(1)
fn encode_tiny_schema() -> Vec<u8> {
    let mut buf = Vec::new();
    // def count = 1
    buf.push(1);
    // name "Point\0"
    buf.extend_from_slice(b"Point\0");
    // kind = 1 (STRUCT)
    buf.push(1);
    // field count = 2
    buf.push(2);
    // field "x": name, type=int(-3 as var_int), not array, value=0
    buf.extend_from_slice(b"x\0");
    // var_int(-3): zigzag encode: (-3 << 1) ^ (-3 >> 31) = (-6) ^ (-1) = 5
    // as unsigned: 5
    buf.push(5); // var_int encoding of -3
    buf.push(0); // not array
    buf.push(0); // value=0
    // field "y": same type
    buf.extend_from_slice(b"y\0");
    buf.push(5); // var_int(-3)
    buf.push(0);
    buf.push(1); // value=1
    buf
}

/// Encode a STRUCT message for Point { x: 42, y: -7 }
/// STRUCT: fields written in order
/// x = 42: var_int(42) = zigzag(42) = 84 -> single byte 0x54
/// y = -7: zigzag(-7) = 13 -> single byte 0x0D
fn encode_point_message() -> Vec<u8> {
    // var_int(42): (42<<1)^(42>>31) = 84 ^ 0 = 84
    // var_int(-7): (-7<<1)^(-7>>31) = -14 ^ -1 = 13
    vec![84, 13]
}

#[test]
fn test_decode_struct() {
    let schema_bytes = encode_tiny_schema();
    let schema = decode_binary_schema(&schema_bytes).expect("schema decode");
    assert_eq!(schema.definitions.len(), 1);
    assert_eq!(schema.definitions[0].name, "Point");
    assert_eq!(schema.definitions[0].kind, DefinitionKind::Struct);
    assert_eq!(schema.definitions[0].fields.len(), 2);
    assert_eq!(schema.definitions[0].fields[0].type_ref, Some(TypeRef::Int));

    let msg_bytes = encode_point_message();
    let val = decode_message("Point", &msg_bytes, &schema).expect("message decode");
    match val {
        Value::Object(map) => {
            assert_eq!(map["x"].as_f32(), Some(42.0));
            // y = -7
            match &map["y"] {
                Value::Int(v) => assert_eq!(*v, -7),
                other => panic!("expected Int, got {other:?}"),
            }
        }
        other => panic!("expected Object, got {other:?}"),
    }
}

#[test]
fn test_var_float_zero() {
    let data = [0u8];
    let mut r = ByteReader::new(&data);
    let v = r.read_var_float().expect("read var_float");
    assert_eq!(v, 0.0f32);
}

#[test]
fn test_var_float_one() {
    // 1.0 bits = 0x3F800000. Write rotation: (bits>>>23)|(bits<<9).
    // (0x3F800000>>>23) = 0x7F; (0x3F800000<<9) = 0 (all significant bits shift past 32).
    // Encoded = 0x7F stored as LE 4 bytes: [0x7F, 0x00, 0x00, 0x00].
    // Read: bits=0x7F, rotate_left(23) = 0x3F800000 = 1.0 ✓
    let data = [0x7F, 0x00, 0x00, 0x00u8];
    let mut r = ByteReader::new(&data);
    let v = r.read_var_float().expect("read var_float 1.0");
    assert!((v - 1.0f32).abs() < 1e-6, "expected 1.0, got {v}");
}

#[test]
fn test_message_decode() {
    // Build a schema with a MESSAGE type: Msg { x: uint @1; s: string @2; }
    // Then encode a message with x=10, s="hi"
    let mut schema_buf = Vec::new();
    schema_buf.push(1); // 1 definition
    schema_buf.extend_from_slice(b"Msg\0");
    schema_buf.push(2); // MESSAGE
    schema_buf.push(2); // 2 fields
    // field x: uint @1
    schema_buf.extend_from_slice(b"x\0");
    // var_int for "uint" -> PRIMITIVES[3], index is ~(-4) = 3? No:
    // primitives: 0=bool,1=byte,2=int,3=uint,4=float,5=string,6=int64,7=uint64
    // for uint: index=3, encoded as type_raw = ~3 in reverse... wait
    // In binary.ts write: type index for "uint" is 3, written as var_int(~3) = var_int(-4)
    // zigzag(-4): (-4<<1)^(-4>>31) = -8 ^ -1 = 7
    schema_buf.push(7); // var_int(-4) = uint
    schema_buf.push(0); // not array
    schema_buf.push(1); // value/tag=1
    // field s: string @2
    schema_buf.extend_from_slice(b"s\0");
    // var_int for "string": index=5 -> type_raw = ~5 = -6 -> zigzag(-6) = 11
    schema_buf.push(11); // var_int(-6) = string
    schema_buf.push(0); // not array
    schema_buf.push(2); // value/tag=2

    let schema = decode_binary_schema(&schema_buf).expect("schema");

    // Encode message: tag=1, x=10, tag=2, s="hi\0", tag=0
    let mut msg_buf = Vec::new();
    msg_buf.push(1); // tag=1
    msg_buf.push(10); // var_uint(10) = 10
    msg_buf.push(2); // tag=2
    msg_buf.extend_from_slice(b"hi\0"); // null-terminated string
    msg_buf.push(0); // end tag

    let val = decode_message("Msg", &msg_buf, &schema).expect("decode");
    match val {
        Value::Object(map) => {
            assert_eq!(map["x"].as_u32(), Some(10));
            assert_eq!(map["s"].as_str(), Some("hi"));
        }
        other => panic!("expected Object, got {other:?}"),
    }
}

// ── ByteWriter: write it, read it back, get it ───────────────────────────────
//
// The whole contract of the writer, tested at the boundaries where a shift or
// a cast goes wrong. No fixture is involved and none could help.

/// Writes with `write`, reads back with `read`, and returns what came out.
macro_rules! round_trip {
    ($write:ident, $read:ident, $value:expr) => {{
        let mut writer = ByteWriter::new();
        writer.$write($value);
        let bytes = writer.into_bytes();
        let mut reader = ByteReader::new(&bytes);
        let out = reader.$read().expect("reads back");
        assert_eq!(
            reader.pos(),
            bytes.len(),
            "{} left {} of {} bytes unread",
            stringify!($write),
            bytes.len() - reader.pos(),
            bytes.len()
        );
        out
    }};
}

#[test]
fn bytes_survive_a_write_and_a_read() {
    for value in [0u8, 1, 0x7F, 0x80, u8::MAX] {
        assert_eq!(round_trip!(write_byte, read_byte, value), value);
    }
}

#[test]
fn unsigned_integers_survive_a_write_and_a_read() {
    for value in [0u32, 1, 127, 128, 16_383, 16_384, u32::MAX - 1, u32::MAX] {
        assert_eq!(round_trip!(write_var_uint, read_var_uint, value), value);
    }
}

/// The zigzag boundary. `read_var_int` used to shift a value it had already
/// cast to `i32`, so everything from 2^30 up came back as a different number
/// entirely — see the comment on the reader.
#[test]
fn signed_integers_survive_a_write_and_a_read() {
    for value in [0i32, 1, -1, 63, -64, 1 << 29, 1 << 30, -(1 << 30), i32::MAX, i32::MIN] {
        assert_eq!(round_trip!(write_var_int, read_var_int, value), value, "value {value}");
    }
}

#[test]
fn wide_unsigned_integers_survive_a_write_and_a_read() {
    for value in [0u64, 1, 127, 128, 1 << 55, 1 << 56, 1 << 63, u64::MAX - 1, u64::MAX] {
        assert_eq!(round_trip!(write_var_uint64, read_var_uint64, value), value, "value {value}");
    }
}

#[test]
fn wide_signed_integers_survive_a_write_and_a_read() {
    for value in [0i64, 1, -1, 1 << 55, -(1 << 55), 1 << 62, i64::MAX, i64::MIN] {
        assert_eq!(round_trip!(write_var_int64, read_var_int64, value), value, "value {value}");
    }
}

#[test]
fn strings_survive_a_write_and_a_read() {
    for value in ["", "a", "hello", "Cabecera", "日本語 — em dash", "🎨"] {
        assert_eq!(round_trip!(write_string, read_string, value), value, "value {value:?}");
    }
}

#[test]
fn byte_arrays_survive_a_write_and_a_read() {
    for value in [vec![], vec![0u8], vec![0u8; 300], (0..=255u8).collect::<Vec<_>>()] {
        assert_eq!(round_trip!(write_byte_array, read_byte_array, &value), value);
    }
}

// ── Floats: what survives, and what does not (research D7) ───────────────────

#[test]
fn ordinary_floats_survive_a_write_and_a_read() {
    for value in [1.0f32, -1.0, 0.5, -0.5, 1e-30, 1e30, f32::MIN, f32::MAX, f32::MIN_POSITIVE] {
        assert_eq!(
            round_trip!(write_var_float, read_var_float, value).to_bits(),
            value.to_bits(),
            "value {value}"
        );
    }
}

/// NaN and the infinities have a full exponent, so they take the four-byte
/// form and come back bit-for-bit.
#[test]
fn nan_and_the_infinities_survive_a_write_and_a_read() {
    for value in [f32::NAN, -f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            round_trip!(write_var_float, read_var_float, value).to_bits(),
            value.to_bits(),
            "value {value}"
        );
    }
}

/// The two that do not, and why that is not a fidelity loss.
///
/// The format stores anything with a zero exponent — zero and the denormals —
/// as a single zero byte, and the reader turns that byte back into `+0.0`. So
/// `-0.0` loses its sign and a denormal loses its magnitude.
///
/// This costs a real file nothing. Figma's own writer made the same choice, so
/// a denormal in a `.fig` arrived as that same single zero byte, decoded to
/// `0.0`, and is written back as a single zero byte. The **bytes** round-trip
/// exactly; it is only a value that never came from a file that could differ.
#[test]
fn negative_zero_and_denormals_do_not_survive_and_that_is_the_format() {
    let denormal = f32::from_bits(1); // the smallest denormal there is

    assert_eq!(round_trip!(write_var_float, read_var_float, -0.0f32).to_bits(), 0.0f32.to_bits());
    assert_eq!(round_trip!(write_var_float, read_var_float, denormal), 0.0);

    // One byte, which is exactly what the reader's zero case expects.
    let mut writer = ByteWriter::new();
    writer.write_var_float(denormal);
    assert_eq!(writer.into_bytes(), vec![0]);
}

// ── The round-trip law ───────────────────────────────────────────────────────
//
// `decode_message(encode_message(v)) == v`, over hand-built schemas. No
// fixture, no real file: the encoder's correctness is established here and is
// not re-litigated once a `.fig` is involved.

fn field(name: &str, type_ref: Option<TypeRef>, is_array: bool, value: u32) -> Field {
    Field { name: name.to_string(), type_ref, is_array, value }
}

fn definition(name: &str, kind: DefinitionKind, fields: Vec<Field>) -> Definition {
    Definition { name: name.to_string(), kind, fields }
}

/// One schema exercising every `DefinitionKind` and every `TypeRef`.
fn every_kind_of_schema() -> Schema {
    use DefinitionKind::{Enum, Message, Struct};
    use TypeRef::{Bool, Byte, Float, Int, Int64, Named, Str, Uint, Uint64};

    Schema {
        definitions: vec![
            definition(
                "Colour",
                Enum,
                // Deliberately not 0,1: an enum's numbers are its own, and the
                // encoder must look them up rather than count.
                vec![field("Red", None, false, 7), field("Green", None, false, 12)],
            ),
            definition(
                "Point",
                Struct,
                vec![field("x", Some(Int), false, 0), field("y", Some(Int), false, 1)],
            ),
            definition("Inner", Message, vec![field("label", Some(Str), false, 1)]),
            definition(
                "Outer",
                Message,
                vec![
                    field("point", Some(Named("Point".into())), false, 1),
                    field("colour", Some(Named("Colour".into())), false, 2),
                    field("inner", Some(Named("Inner".into())), false, 3),
                    field("points", Some(Named("Point".into())), true, 4),
                    field("data", Some(Byte), true, 5),
                    field("flag", Some(Bool), false, 6),
                    field("tiny", Some(Byte), false, 7),
                    field("big", Some(Uint64), false, 8),
                    field("wide", Some(Int64), false, 9),
                    field("ratio", Some(Float), false, 10),
                    field("names", Some(Str), true, 11),
                    field("count", Some(Uint), false, 12),
                ],
            ),
        ],
    }
}

fn object(fields: Vec<(&str, Value)>) -> Value {
    Value::Object(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn point(x: i32, y: i32) -> Value {
    object(vec![("x", Value::Int(x)), ("y", Value::Int(y))])
}

/// Encodes, decodes, and asserts the law.
#[track_caller]
fn survives(root: &str, value: &Value, schema: &Schema) {
    let bytes = encode_message(root, value, schema).expect("encodes");
    let back = decode_message(root, &bytes, schema).expect("decodes");
    assert_eq!(&back, value);
}

#[test]
fn every_value_kind_survives_an_encode_and_a_decode() {
    let schema = every_kind_of_schema();

    survives("Point", &point(3, -4), &schema);
    survives("Colour", &Value::Enum("Green"), &schema);
    survives("Inner", &object(vec![("label", Value::Str("hello".into()))]), &schema);

    survives(
        "Outer",
        &object(vec![
            ("point", point(1, 2)),
            ("colour", Value::Enum("Red")),
            ("inner", object(vec![("label", Value::Str("nested".into()))])),
            ("points", Value::Array(vec![point(0, 0), point(-1, 1 << 30)])),
            ("data", Value::Bytes(vec![0, 1, 255])),
            ("flag", Value::Bool(true)),
            // A byte above 127, which a varint would have written as two.
            ("tiny", Value::Uint(200)),
            ("big", Value::Uint64(u64::MAX)),
            ("wide", Value::Int64(i64::MIN)),
            ("ratio", Value::Float(1.5)),
            ("names", Value::Array(vec![Value::Str(String::new()), Value::Str("x".into())])),
            ("count", Value::Uint(u32::MAX)),
        ]),
        &schema,
    );
}

/// An empty message is a lone terminator, and an empty array is a lone zero.
#[test]
fn empty_messages_and_empty_arrays_survive() {
    let schema = every_kind_of_schema();

    survives("Outer", &object(vec![]), &schema);
    assert_eq!(encode_message("Outer", &object(vec![]), &schema).expect("encodes"), vec![0]);

    survives("Outer", &object(vec![("points", Value::Array(vec![]))]), &schema);
    survives("Outer", &object(vec![("data", Value::Bytes(vec![]))]), &schema);
}

/// A message may carry any subset of its fields, in any order — the tag says
/// which. That is what makes the next test meaningful.
#[test]
fn a_message_written_out_of_tag_order_survives() {
    let schema = every_kind_of_schema();
    survives(
        "Outer",
        &object(vec![
            ("count", Value::Uint(9)),
            ("flag", Value::Bool(false)),
            ("point", point(5, 6)),
        ]),
        &schema,
    );
}

/// D4: the decoded map holds the file's tag order, so re-encoding reproduces
/// the file's tag *sequence* — not a tidied, ascending one.
#[test]
fn re_encoding_reproduces_the_original_tag_order_and_not_a_sorted_one() {
    let schema = Schema {
        definitions: vec![definition(
            "Pair",
            DefinitionKind::Message,
            vec![
                field("a", Some(TypeRef::Uint), false, 1),
                field("b", Some(TypeRef::Uint), false, 2),
            ],
        )],
    };

    // Tag 2 first, then tag 1 — descending, which Kiwi permits.
    let original = vec![2, 20, 1, 10, 0];

    let decoded = decode_message("Pair", &original, &schema).expect("decodes");
    let Value::Object(map) = &decoded else { panic!("a message decodes to an object") };
    assert_eq!(
        map.keys().collect::<Vec<_>>(),
        vec!["b", "a"],
        "the IndexMap records the order the bytes were in"
    );

    assert_eq!(
        encode_message("Pair", &decoded, &schema).expect("encodes"),
        original,
        "matching the input costs nothing here and removes a class of difference"
    );
}

// ── Failing loudly ───────────────────────────────────────────────────────────
//
// A `Value` that does not fit its definition is a bug in whatever built it.
// Each of these must name the definition and the field, so that a failure is a
// place to go and look rather than an investigation.

#[test]
fn a_value_that_does_not_fit_its_definition_is_an_error_that_says_where() {
    let schema = every_kind_of_schema();

    let missing = encode_message("Point", &object(vec![("x", Value::Int(1))]), &schema)
        .expect_err("a struct field cannot be skipped");
    assert!(matches!(&missing, KiwiError::MissingField { def, field } if def == "Point" && field == "y"));
    assert_eq!(missing.to_string(), "Point.y is missing");

    let wrong_type = encode_message(
        "Point",
        &object(vec![("x", Value::Str("one".into())), ("y", Value::Int(2))]),
        &schema,
    )
    .expect_err("a string is not an int");
    assert_eq!(wrong_type.to_string(), "Point.x: expected an int, found a string");

    let unknown_enum = encode_message("Colour", &Value::Enum("Blue"), &schema)
        .expect_err("the definition has no Blue");
    assert_eq!(unknown_enum.to_string(), "unknown enum name Blue in Colour");

    let unknown_field = encode_message("Outer", &object(vec![("nope", Value::Uint(1))]), &schema)
        .expect_err("the schema defines no such tag");
    assert_eq!(unknown_field.to_string(), "Outer has no field named nope");

    let not_an_object = encode_message("Point", &Value::Uint(1), &schema)
        .expect_err("a struct is not a scalar");
    assert_eq!(not_an_object.to_string(), "expected an object for Point, found a uint");

    // Silently truncating this is exactly the kind of wrongness that only
    // surfaces when Figma refuses the file.
    let too_big = encode_message("Outer", &object(vec![("tiny", Value::Uint(256))]), &schema)
        .expect_err("256 is not a byte");
    assert_eq!(too_big.to_string(), "Outer.tiny: 256 does not fit in a byte");
}

// ── Fields and interned names ──────────────────────────────────

#[test]
fn fields_keep_order_replace_in_place_and_remove_shifting() {
    let mut fields = Fields::new();
    fields.insert("b", Value::Uint(1));
    fields.insert(String::from("a"), Value::Uint(2));
    fields.insert("c", Value::Uint(3));
    assert_eq!(fields.insert("a", Value::Uint(20)), Some(Value::Uint(2)));
    assert_eq!(fields.keys().collect::<Vec<_>>(), ["b", "a", "c"], "a replaced value keeps its place");
    assert_eq!(fields.shift_remove("b"), Some(Value::Uint(1)));
    assert_eq!(fields.keys().collect::<Vec<_>>(), ["a", "c"]);
    assert_eq!(fields.get("a"), Some(&Value::Uint(20)));
    assert!(fields.get("b").is_none());
    *fields.get_or_insert_with("d", || Value::Uint(0)) = Value::Uint(4);
    assert_eq!(fields["d"], Value::Uint(4));
    assert_eq!(fields.len(), 3);
}

#[test]
fn a_name_is_stored_once_however_many_objects_use_it() {
    let one = intern(&String::from("fillPaints"));
    let two = intern("fillPaints");
    assert!(std::ptr::eq(one, two));
    let (mut a, mut b) = (Fields::new(), Fields::new());
    a.insert(String::from("strokeWeight"), Value::Float(1.0));
    b.insert("strokeWeight", Value::Float(2.0));
    assert!(std::ptr::eq(a.keys().next().unwrap(), b.keys().next().unwrap()));
}

type SchemaDef<'a> = (&'a str, u8, &'a [(&'a str, i32, bool)]);

/// A binary schema: each definition's name, kind and fields, each field's name,
/// type (a definition's index, or a primitive as a negative) and whether it is an
/// array.
fn schema_of(defs: &[SchemaDef<'_>]) -> Schema {
    fn var_uint(buf: &mut Vec<u8>, mut value: u32) {
        while value >= 0x80 {
            buf.push((value as u8 & 0x7F) | 0x80);
            value >>= 7;
        }
        buf.push(value as u8);
    }
    let mut buf = Vec::new();
    var_uint(&mut buf, defs.len() as u32);
    for (name, kind, fields) in defs {
        buf.extend_from_slice(name.as_bytes());
        buf.push(0);
        buf.push(*kind);
        var_uint(&mut buf, fields.len() as u32);
        for (index, (field, ty, array)) in fields.iter().enumerate() {
            buf.extend_from_slice(field.as_bytes());
            buf.push(0);
            var_uint(&mut buf, ((ty << 1) ^ (ty >> 31)) as u32);
            buf.push(u8::from(*array));
            var_uint(&mut buf, index as u32 + 1);
        }
    }
    decode_binary_schema(&buf).expect("a schema")
}

const STRUCT: u8 = 1;
const MESSAGE: u8 = 2;

#[test]
fn a_struct_holding_itself_is_refused_with_no_input_read() {
    let schema = schema_of(&[("Message", MESSAGE, &[("loop", 1, false)]), ("Loop", STRUCT, &[("next", 1, false)])]);
    let error = decode_message("Message", &[1, 0], &schema).unwrap_err();
    assert!(matches!(error, KiwiError::TooDeep { limit: 64 }), "{error}");
    assert!(error.to_string().contains("64"), "{error}");
}

#[test]
fn a_message_nested_past_the_limit_is_refused_and_one_at_it_is_read() {
    let schema = schema_of(&[("Message", MESSAGE, &[("child", 0, false)])]);
    // Each level opens with its tag, and closes with a zero.
    let nested = |levels: usize| [vec![1u8; levels - 1], vec![0u8; levels]].concat();
    assert!(decode_message("Message", &nested(64), &schema).is_ok());
    let error = decode_message("Message", &nested(65), &schema).unwrap_err();
    assert!(matches!(error, KiwiError::TooDeep { .. }), "{error}");
}

#[test]
fn empty_structs_cost_no_input_and_are_counted() {
    let schema = schema_of(&[("Message", MESSAGE, &[("items", 1, true)]), ("Item", STRUCT, &[])]);
    // 40,000,000 as a var_uint.
    let error = decode_message("Message", &[1, 0x80, 0xB4, 0x89, 0x13, 0], &schema).unwrap_err();
    assert!(matches!(error, KiwiError::TooManyValues { limit: 1030 }), "{error}");
}

#[test]
fn structs_doubling_at_each_level_are_refused_on_their_count() {
    // 32 levels would be 2^32 values, within the nesting limit, from no input.
    let names: Vec<String> = (0..32).map(|level| format!("S{level}")).collect();
    let fields: Vec<[(&str, i32, bool); 2]> =
        (0..32).map(|level| [("a", level + 2, false), ("b", level + 2, false)]).collect();
    let mut defs: Vec<SchemaDef<'_>> = vec![("Message", MESSAGE, &[("top", 1, false)])];
    for (level, name) in names.iter().enumerate() {
        defs.push((name.as_str(), STRUCT, if level == 31 { &[] } else { &fields[level] }));
    }
    let error = decode_message("Message", &[1, 0], &schema_of(&defs)).unwrap_err();
    assert!(matches!(error, KiwiError::TooManyValues { .. }), "{error}");
}

#[test]
fn a_length_past_the_input_reserves_nothing_and_ends_the_input() {
    let schema = schema_of(&[("Message", MESSAGE, &[("items", 1, true)]), ("Item", STRUCT, &[("a", -3, false)])]);
    let all_of_u32 = [1, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F];
    assert!(matches!(decode_message("Message", &all_of_u32, &schema), Err(KiwiError::UnexpectedEof)));
    // A schema declaring as many definitions.
    assert!(matches!(decode_binary_schema(&all_of_u32[1..]), Err(KiwiError::UnexpectedEof)));
}
