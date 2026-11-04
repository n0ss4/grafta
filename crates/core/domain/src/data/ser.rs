//! Any value serde can describe, as `Data`: what `serde_json::to_value` does,
//! case for case, so a result built with [`data!`](super::data) is the one
//! `json!` built.

use std::fmt;

use serde::ser::{self, Impossible, Serialize};

use super::{Data, Map};

/// Why a value has no `Data`: a map key that is not text, or a number out of
/// range, as `serde_json` refuses them.
#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self(message.to_string())
    }
}

fn key_must_be_a_string() -> Error {
    Error("key must be a string".into())
}

/// `value` as `Data`.
pub fn to_data<T: Serialize + ?Sized>(value: &T) -> Result<Data, Error> {
    value.serialize(Serializer)
}

struct Serializer;

impl ser::Serializer for Serializer {
    type Ok = Data;
    type Error = Error;
    type SerializeSeq = List;
    type SerializeTuple = List;
    type SerializeTupleStruct = List;
    type SerializeTupleVariant = Variant<List>;
    type SerializeMap = Entries;
    type SerializeStruct = Entries;
    type SerializeStructVariant = Variant<Entries>;

    fn serialize_bool(self, v: bool) -> Result<Data, Error> {
        Ok(Data::Bool(v))
    }
    fn serialize_i8(self, v: i8) -> Result<Data, Error> {
        Ok(Data::from(i64::from(v)))
    }
    fn serialize_i16(self, v: i16) -> Result<Data, Error> {
        Ok(Data::from(i64::from(v)))
    }
    fn serialize_i32(self, v: i32) -> Result<Data, Error> {
        Ok(Data::from(i64::from(v)))
    }
    fn serialize_i64(self, v: i64) -> Result<Data, Error> {
        Ok(Data::from(v))
    }
    fn serialize_i128(self, v: i128) -> Result<Data, Error> {
        if let Ok(v) = u64::try_from(v) {
            Ok(Data::from(v))
        } else if let Ok(v) = i64::try_from(v) {
            Ok(Data::from(v))
        } else {
            Err(Error("number out of range".into()))
        }
    }
    fn serialize_u8(self, v: u8) -> Result<Data, Error> {
        Ok(Data::from(u64::from(v)))
    }
    fn serialize_u16(self, v: u16) -> Result<Data, Error> {
        Ok(Data::from(u64::from(v)))
    }
    fn serialize_u32(self, v: u32) -> Result<Data, Error> {
        Ok(Data::from(u64::from(v)))
    }
    fn serialize_u64(self, v: u64) -> Result<Data, Error> {
        Ok(Data::from(v))
    }
    fn serialize_u128(self, v: u128) -> Result<Data, Error> {
        u64::try_from(v).map(Data::from).map_err(|_| Error("number out of range".into()))
    }
    /// The `f64` it holds, as `serde_json` keeps it.
    fn serialize_f32(self, v: f32) -> Result<Data, Error> {
        Ok(Data::from(f64::from(v)))
    }
    fn serialize_f64(self, v: f64) -> Result<Data, Error> {
        Ok(Data::from(v))
    }
    fn serialize_char(self, v: char) -> Result<Data, Error> {
        Ok(Data::String(v.to_string()))
    }
    fn serialize_str(self, v: &str) -> Result<Data, Error> {
        Ok(Data::String(v.to_owned()))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Data, Error> {
        Ok(Data::List(v.iter().map(|b| Data::from(u64::from(*b))).collect()))
    }
    fn serialize_none(self) -> Result<Data, Error> {
        Ok(Data::Null)
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Data, Error> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<Data, Error> {
        Ok(Data::Null)
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Data, Error> {
        Ok(Data::Null)
    }
    fn serialize_unit_variant(self, _name: &'static str, _index: u32, variant: &'static str) -> Result<Data, Error> {
        Ok(Data::String(variant.to_owned()))
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(self, _name: &'static str, value: &T) -> Result<Data, Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Data, Error> {
        Ok(Data::Map(Map::from([(variant.to_owned(), to_data(value)?)])))
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<List, Error> {
        Ok(List(Vec::with_capacity(len.unwrap_or(0))))
    }
    fn serialize_tuple(self, len: usize) -> Result<List, Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<List, Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(self, _name: &'static str, _index: u32, variant: &'static str, len: usize) -> Result<Variant<List>, Error> {
        Ok(Variant { name: variant, inner: List(Vec::with_capacity(len)) })
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Entries, Error> {
        Ok(Entries { map: Map::new(), next: None })
    }
    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Entries, Error> {
        self.serialize_map(Some(len))
    }
    fn serialize_struct_variant(self, _name: &'static str, _index: u32, variant: &'static str, _len: usize) -> Result<Variant<Entries>, Error> {
        Ok(Variant { name: variant, inner: Entries { map: Map::new(), next: None } })
    }
}

struct List(Vec<Data>);

impl ser::SerializeSeq for List {
    type Ok = Data;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.0.push(to_data(value)?);
        Ok(())
    }
    fn end(self) -> Result<Data, Error> {
        Ok(Data::List(self.0))
    }
}

impl ser::SerializeTuple for List {
    type Ok = Data;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<Data, Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for List {
    type Ok = Data;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<Data, Error> {
        ser::SerializeSeq::end(self)
    }
}

struct Entries {
    map: Map,
    next: Option<String>,
}

impl ser::SerializeMap for Entries {
    type Ok = Data;
    type Error = Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        self.next = Some(key.serialize(KeySerializer)?);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        let key = self.next.take().expect("serialize_value called before serialize_key");
        self.map.insert(key, to_data(value)?);
        Ok(())
    }
    fn end(self) -> Result<Data, Error> {
        Ok(Data::Map(self.map))
    }
}

impl ser::SerializeStruct for Entries {
    type Ok = Data;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), Error> {
        self.map.insert(key.to_owned(), to_data(value)?);
        Ok(())
    }
    fn end(self) -> Result<Data, Error> {
        Ok(Data::Map(self.map))
    }
}

/// An enum's variant holding a list or fields: `{ variant: … }`.
struct Variant<T> {
    name: &'static str,
    inner: T,
}

impl<T> Variant<T> {
    fn wrap(name: &'static str, value: Data) -> Data {
        Data::Map(Map::from([(name.to_owned(), value)]))
    }
}

impl ser::SerializeTupleVariant for Variant<List> {
    type Ok = Data;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(&mut self.inner, value)
    }
    fn end(self) -> Result<Data, Error> {
        Ok(Self::wrap(self.name, Data::List(self.inner.0)))
    }
}

impl ser::SerializeStructVariant for Variant<Entries> {
    type Ok = Data;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), Error> {
        ser::SerializeStruct::serialize_field(&mut self.inner, key, value)
    }
    fn end(self) -> Result<Data, Error> {
        Ok(Self::wrap(self.name, Data::Map(self.inner.map)))
    }
}

/// A map key as text: a string, a number, a bool or a char, as `serde_json`
/// writes one.
struct KeySerializer;

impl ser::Serializer for KeySerializer {
    type Ok = String;
    type Error = Error;
    type SerializeSeq = Impossible<String, Error>;
    type SerializeTuple = Impossible<String, Error>;
    type SerializeTupleStruct = Impossible<String, Error>;
    type SerializeTupleVariant = Impossible<String, Error>;
    type SerializeMap = Impossible<String, Error>;
    type SerializeStruct = Impossible<String, Error>;
    type SerializeStructVariant = Impossible<String, Error>;

    fn serialize_bool(self, v: bool) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i8(self, v: i8) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i16(self, v: i16) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i32(self, v: i32) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i64(self, v: i64) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i128(self, v: i128) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u8(self, v: u8) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u16(self, v: u16) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u32(self, v: u32) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u64(self, v: u64) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u128(self, v: u128) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_f32(self, v: f32) -> Result<String, Error> {
        if v.is_finite() {
            Ok(zmij::Buffer::new().format_finite(v).to_owned())
        } else {
            Err(Error("float key must be finite".into()))
        }
    }
    fn serialize_f64(self, v: f64) -> Result<String, Error> {
        if v.is_finite() {
            Ok(zmij::Buffer::new().format_finite(v).to_owned())
        } else {
            Err(Error("float key must be finite".into()))
        }
    }
    fn serialize_char(self, v: char) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_str(self, v: &str) -> Result<String, Error> {
        Ok(v.to_owned())
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }
    fn serialize_none(self) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, _value: &T) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }
    fn serialize_unit(self) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }
    fn serialize_unit_variant(self, _name: &'static str, _index: u32, variant: &'static str) -> Result<String, Error> {
        Ok(variant.to_owned())
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(self, _name: &'static str, value: &T) -> Result<String, Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(self, _name: &'static str, _index: u32, _variant: &'static str, _value: &T) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Err(key_must_be_a_string())
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
        Err(key_must_be_a_string())
    }
    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct, Error> {
        Err(key_must_be_a_string())
    }
    fn serialize_tuple_variant(self, _name: &'static str, _index: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeTupleVariant, Error> {
        Err(key_must_be_a_string())
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Err(key_must_be_a_string())
    }
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Error> {
        Err(key_must_be_a_string())
    }
    fn serialize_struct_variant(self, _name: &'static str, _index: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeStructVariant, Error> {
        Err(key_must_be_a_string())
    }
}
