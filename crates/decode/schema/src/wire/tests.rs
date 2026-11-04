use crate::WireName;

crate::wire! {
    enum Sample {
        One = "ONE",
        Two = "TWO",
    }
}
super::wire_value!(Sample);

#[test]
fn a_name_reads_back_as_itself() {
    for name in Sample::ALL {
        assert_eq!(Sample::from_wire(name.as_wire()), Some(*name));
    }
}

#[test]
fn a_name_not_listed_is_none() {
    assert_eq!(Sample::from_wire("THREE"), None);
}

#[test]
fn a_value_is_written_as_its_name() {
    assert_eq!(kiwi::Value::from(Sample::Two), kiwi::Value::Enum("TWO"));
}

#[test]
fn a_field_is_read_as_a_value_grafta_lists_or_not() {
    use crate::{object, Field, Wire, Wired};
    let known = object([(Field::Type, Sample::One.into())]);
    let unknown = object([(Field::Type, kiwi::Value::Enum("THREE"))]);
    assert_eq!(known.known::<Sample>(Field::Type), Some(Sample::One));
    assert!(known.is(Field::Type, Sample::One));
    assert_eq!(unknown.known::<Sample>(Field::Type), None);
    assert_eq!(unknown.enum_of::<Sample>(Field::Type), Some(Wired::Unknown("THREE")));
}

#[test]
fn a_name_is_chosen_from_those_allowed() {
    assert_eq!(Sample::listed("TWO", &[Sample::One, Sample::Two]), Some(Sample::Two));
    assert_eq!(Sample::listed("TWO", &[Sample::One]), None);
    assert_eq!(Sample::joined(&[Sample::One, Sample::Two], ", "), "ONE, TWO");
}
