//! A decoded message read and written by [`Field`], never by string.

use kiwi::{Fields, Value};

use crate::{Field, Guid, WireName};

const EMPTY: &[Value] = &[];

/// A value of one of the format's enums: one grafta lists, or one it does not,
/// kept as the file spells it so it can be reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wired<'a, E> {
    Known(E),
    Unknown(&'a str),
}

impl<E: Copy> Wired<'_, E> {
    #[must_use]
    pub fn known(self) -> Option<E> {
        match self {
            Self::Known(value) => Some(value),
            Self::Unknown(_) => None,
        }
    }
}

pub trait Wire {
    fn field(&self, field: Field) -> Option<&Value>;

    fn has(&self, field: Field) -> bool {
        self.field(field).is_some()
    }

    fn f32(&self, field: Field) -> Option<f32> {
        self.field(field).and_then(Value::as_f32)
    }

    fn u32(&self, field: Field) -> Option<u32> {
        self.field(field).and_then(Value::as_u32)
    }

    fn bool(&self, field: Field) -> Option<bool> {
        self.field(field).and_then(Value::as_bool)
    }

    fn str(&self, field: Field) -> Option<&str> {
        self.field(field).and_then(Value::as_str)
    }

    /// The field's items; none when it is absent or not an array.
    fn array(&self, field: Field) -> &[Value] {
        self.field(field).and_then(Value::as_array).map_or(EMPTY, Vec::as_slice)
    }

    /// A GUID, bare or in its `{ guid }` envelope.
    fn guid(&self, field: Field) -> Option<Guid> {
        self.field(field).and_then(Guid::read_wrapped)
    }

    fn enum_of<E: WireName>(&self, field: Field) -> Option<Wired<'_, E>> {
        let wire = self.str(field)?;
        Some(E::from_wire(wire).map_or(Wired::Unknown(wire), Wired::Known))
    }

    /// The field's value when grafta lists it; `None` when the field is absent
    /// or holds one grafta does not list.
    fn known<E: WireName>(&self, field: Field) -> Option<E> {
        self.enum_of(field).and_then(Wired::known)
    }

    fn is<E: WireName>(&self, field: Field, value: E) -> bool {
        self.str(field) == Some(value.as_wire())
    }
}

pub trait WireMut: Wire {
    fn field_mut(&mut self, field: Field) -> Option<&mut Value>;

    /// Set a field, in place when it exists; the old value when there was one.
    fn set(&mut self, field: Field, value: Value) -> Option<Value>;

    fn take(&mut self, field: Field) -> Option<Value>;

    fn field_or_insert_with(&mut self, field: Field, make: impl FnOnce() -> Value) -> &mut Value;
}

impl Wire for Fields {
    fn field(&self, field: Field) -> Option<&Value> {
        self.get(field.as_wire())
    }
}

impl WireMut for Fields {
    fn field_mut(&mut self, field: Field) -> Option<&mut Value> {
        self.get_mut(field.as_wire())
    }

    fn set(&mut self, field: Field, value: Value) -> Option<Value> {
        self.insert(field.as_wire(), value)
    }

    fn take(&mut self, field: Field) -> Option<Value> {
        Fields::remove(self, field.as_wire())
    }

    fn field_or_insert_with(&mut self, field: Field, make: impl FnOnce() -> Value) -> &mut Value {
        self.get_or_insert_with(field.as_wire(), make)
    }
}

impl Wire for Value {
    fn field(&self, field: Field) -> Option<&Value> {
        self.get(field.as_wire())
    }
}

/// An object's fields; `None` on a value that is not an object.
fn fields_mut(value: &mut Value) -> Option<&mut Fields> {
    match value {
        Value::Object(fields) => Some(fields),
        _ => None,
    }
}

impl WireMut for Value {
    fn field_mut(&mut self, field: Field) -> Option<&mut Value> {
        fields_mut(self)?.field_mut(field)
    }

    /// On a value that is not an object, nothing is set.
    fn set(&mut self, field: Field, value: Value) -> Option<Value> {
        fields_mut(self)?.set(field, value)
    }

    fn take(&mut self, field: Field) -> Option<Value> {
        fields_mut(self)?.take(field)
    }

    /// A value that is not an object becomes one first.
    fn field_or_insert_with(&mut self, field: Field, make: impl FnOnce() -> Value) -> &mut Value {
        if !matches!(self, Value::Object(_)) {
            *self = Value::Object(Fields::new());
        }
        let Value::Object(fields) = self else { unreachable!("made an object above") };
        fields.field_or_insert_with(field, make)
    }
}

/// These fields, in this order.
#[must_use]
pub fn fields<const N: usize>(fields: [(Field, Value); N]) -> Fields {
    fields.into_iter().map(|(field, value)| (field.as_wire(), value)).collect()
}

/// An object of these fields, in this order.
#[must_use]
pub fn object<const N: usize>(entries: [(Field, Value); N]) -> Value {
    Value::Object(fields(entries))
}
