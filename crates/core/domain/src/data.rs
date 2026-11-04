//! A value whose shape the file or the model decides: the properties a call
//! sets, the layout it imports, what a node shows.
//!
//! It holds what a JSON value holds, and no more: a map's keys sorted, as
//! grafta's JSON has always had them, and an integer kept apart from a float,
//! so `1` and `1.0` read back as they came. Written with `{}`, it is the text
//! the model sent, byte for byte, which is how a message quotes a bad value.

use std::collections::BTreeMap;
use std::fmt::{self, Write};
use std::ops::{Index, IndexMut};

mod ser;

use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

pub use ser::{to_data, Error};

/// A map of `Data`, its keys sorted.
pub type Map = BTreeMap<String, Data>;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Data {
    #[default]
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    List(Vec<Data>),
    Map(BTreeMap<String, Data>),
}

/// A number as it was written: a whole number, signed or not, or a float.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    Unsigned(u64),
    Signed(i64),
    Float(f64),
}

static NULL: Data = Data::Null;

impl Data {
    /// A map's entry, or `None` when it is absent or this is not a map.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Data> {
        self.as_map().and_then(|map| map.get(key))
    }

    #[must_use]
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Data> {
        self.as_map_mut().and_then(|map| map.get_mut(key))
    }

    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    #[must_use]
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    /// Any number, as a float.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(Number::Unsigned(n)) => Some(*n as f64),
            Self::Number(Number::Signed(n)) => Some(*n as f64),
            Self::Number(Number::Float(n)) => Some(*n),
            _ => None,
        }
    }

    /// A whole number that is not negative; a float is not one, even `1.0`.
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(Number::Unsigned(n)) => Some(*n),
            Self::Number(Number::Signed(n)) => u64::try_from(*n).ok(),
            _ => None,
        }
    }

    /// A whole number; a float is not one, even `1.0`.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(Number::Unsigned(n)) => i64::try_from(*n).ok(),
            Self::Number(Number::Signed(n)) => Some(*n),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_list(&self) -> Option<&Vec<Data>> {
        match self {
            Self::List(list) => Some(list),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_list(&self) -> bool {
        matches!(self, Self::List(_))
    }

    #[must_use]
    pub fn as_map(&self) -> Option<&BTreeMap<String, Data>> {
        match self {
            Self::Map(map) => Some(map),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_map_mut(&mut self) -> Option<&mut BTreeMap<String, Data>> {
        match self {
            Self::Map(map) => Some(map),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_map(&self) -> bool {
        matches!(self, Self::Map(_))
    }
}

/// `Data` described to serde as the JSON it is, so the edge writes one
/// directly.
impl Serialize for Data {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Data::Null => serializer.serialize_unit(),
            Data::Bool(b) => serializer.serialize_bool(*b),
            Data::Number(Number::Unsigned(n)) => serializer.serialize_u64(*n),
            Data::Number(Number::Signed(n)) => serializer.serialize_i64(*n),
            Data::Number(Number::Float(n)) => serializer.serialize_f64(*n),
            Data::String(s) => serializer.serialize_str(s),
            Data::List(list) => {
                let mut seq = serializer.serialize_seq(Some(list.len()))?;
                for item in list {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Data::Map(map) => {
                let mut out = serializer.serialize_map(Some(map.len()))?;
                for (key, value) in map {
                    out.serialize_entry(key, value)?;
                }
                out.end()
            }
        }
    }
}

/// `data["key"]` and `data[0]` read as JSON's do: what is absent is null.
impl Index<&str> for Data {
    type Output = Data;
    fn index(&self, key: &str) -> &Data {
        self.get(key).unwrap_or(&NULL)
    }
}

/// `data["key"] = value` writes as JSON's does: a null becomes a map, and a
/// key not there is added.
impl IndexMut<&str> for Data {
    fn index_mut(&mut self, key: &str) -> &mut Data {
        if self.is_null() {
            *self = Self::Map(Map::new());
        }
        match self {
            Self::Map(map) => map.entry(key.to_owned()).or_insert(Self::Null),
            other => panic!("cannot index {other} with {key:?}"),
        }
    }
}

impl Index<usize> for Data {
    type Output = Data;
    fn index(&self, at: usize) -> &Data {
        self.as_list().and_then(|list| list.get(at)).unwrap_or(&NULL)
    }
}

/// `data == "x"`, `data == 3`, as JSON's compare: a number by its value read
/// as the type it is compared with.
impl PartialEq<str> for Data {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == Some(other)
    }
}

impl PartialEq<&str> for Data {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == Some(*other)
    }
}

impl PartialEq<String> for Data {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == Some(other.as_str())
    }
}

impl PartialEq<bool> for Data {
    fn eq(&self, other: &bool) -> bool {
        self.as_bool() == Some(*other)
    }
}

macro_rules! compared_as {
    ($read:ident as $wide:ty: $($t:ty),*) => {$(
        impl PartialEq<$t> for Data {
            #[allow(clippy::cast_lossless)]
            fn eq(&self, other: &$t) -> bool {
                self.$read() == Some(*other as $wide)
            }
        }
    )*};
}

compared_as!(as_i64 as i64: i8, i16, i32, i64, isize);
compared_as!(as_u64 as u64: u8, u16, u32, u64, usize);
compared_as!(as_f64 as f64: f32, f64);

impl From<&str> for Data {
    fn from(s: &str) -> Self {
        Self::String(s.into())
    }
}

impl From<String> for Data {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<bool> for Data {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<u64> for Data {
    fn from(n: u64) -> Self {
        Self::Number(Number::Unsigned(n))
    }
}

impl From<i64> for Data {
    /// Not negative is unsigned, as a JSON reader would have it.
    fn from(n: i64) -> Self {
        Self::Number(u64::try_from(n).map_or(Number::Signed(n), Number::Unsigned))
    }
}

impl From<f64> for Data {
    /// A float that is not finite has no JSON, and is null.
    fn from(n: f64) -> Self {
        if n.is_finite() {
            Self::Number(Number::Float(n))
        } else {
            Self::Null
        }
    }
}

impl From<Vec<Data>> for Data {
    fn from(list: Vec<Data>) -> Self {
        Self::List(list)
    }
}

impl From<BTreeMap<String, Data>> for Data {
    fn from(map: BTreeMap<String, Data>) -> Self {
        Self::Map(map)
    }
}

/// Compact, as `serde_json` writes it: the same escapes, and a float by the
/// same formatter.
impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Number(n) => write!(f, "{n}"),
            Self::String(s) => quoted(f, s),
            Self::List(list) => {
                f.write_char('[')?;
                for (at, item) in list.iter().enumerate() {
                    if at > 0 {
                        f.write_char(',')?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_char(']')
            }
            Self::Map(map) => {
                f.write_char('{')?;
                for (at, (key, value)) in map.iter().enumerate() {
                    if at > 0 {
                        f.write_char(',')?;
                    }
                    quoted(f, key)?;
                    write!(f, ":{value}")?;
                }
                f.write_char('}')
            }
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsigned(n) => write!(f, "{n}"),
            Self::Signed(n) => write!(f, "{n}"),
            Self::Float(n) if n.is_finite() => f.write_str(zmij::Buffer::new().format_finite(*n)),
            Self::Float(_) => f.write_str("null"),
        }
    }
}

fn quoted(f: &mut fmt::Formatter<'_>, s: &str) -> fmt::Result {
    f.write_char('"')?;
    for c in s.chars() {
        match c {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\r' => f.write_str("\\r")?,
            '\t' => f.write_str("\\t")?,
            '\u{08}' => f.write_str("\\b")?,
            '\u{0C}' => f.write_str("\\f")?,
            c if u32::from(c) < 0x20 => write!(f, "\\u{:04x}", u32::from(c))?,
            c => f.write_char(c)?,
        }
    }
    f.write_char('"')
}

/// A `Data` written as `json!` writes JSON: `data!({ "id": id, "set": [] })`.
/// A value is anything serde can describe, as [`to_data`] reads it.
#[macro_export]
macro_rules! data {
    // A list's items, read one at a time into `[$($items,)*]`.
    (@list [$($items:expr,)*]) => { vec![$($items,)*] };
    (@list [$($items:expr),*]) => { vec![$($items),*] };
    (@list [$($items:expr,)*] null $($rest:tt)*) => { $crate::data!(@list [$($items,)* $crate::data!(null)] $($rest)*) };
    (@list [$($items:expr,)*] true $($rest:tt)*) => { $crate::data!(@list [$($items,)* $crate::data!(true)] $($rest)*) };
    (@list [$($items:expr,)*] false $($rest:tt)*) => { $crate::data!(@list [$($items,)* $crate::data!(false)] $($rest)*) };
    (@list [$($items:expr,)*] [$($list:tt)*] $($rest:tt)*) => { $crate::data!(@list [$($items,)* $crate::data!([$($list)*])] $($rest)*) };
    (@list [$($items:expr,)*] {$($map:tt)*} $($rest:tt)*) => { $crate::data!(@list [$($items,)* $crate::data!({$($map)*})] $($rest)*) };
    (@list [$($items:expr,)*] $next:expr, $($rest:tt)*) => { $crate::data!(@list [$($items,)* $crate::data!($next),] $($rest)*) };
    (@list [$($items:expr,)*] $last:expr) => { $crate::data!(@list [$($items,)* $crate::data!($last)]) };
    (@list [$($items:expr),*] , $($rest:tt)*) => { $crate::data!(@list [$($items,)*] $($rest)*) };

    // A map's entries: the key's tokens gathered in (), then its value.
    (@map $map:ident () () ()) => {};
    (@map $map:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
        let _ = $map.insert(($($key)+).into(), $value);
        $crate::data!(@map $map () ($($rest)*) ($($rest)*));
    };
    (@map $map:ident [$($key:tt)+] ($value:expr)) => {
        let _ = $map.insert(($($key)+).into(), $value);
    };
    (@map $map:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => { $crate::data!(@map $map [$($key)+] ($crate::data!(null)) $($rest)*); };
    (@map $map:ident ($($key:tt)+) (: true $($rest:tt)*) $copy:tt) => { $crate::data!(@map $map [$($key)+] ($crate::data!(true)) $($rest)*); };
    (@map $map:ident ($($key:tt)+) (: false $($rest:tt)*) $copy:tt) => { $crate::data!(@map $map [$($key)+] ($crate::data!(false)) $($rest)*); };
    (@map $map:ident ($($key:tt)+) (: [$($list:tt)*] $($rest:tt)*) $copy:tt) => { $crate::data!(@map $map [$($key)+] ($crate::data!([$($list)*])) $($rest)*); };
    (@map $map:ident ($($key:tt)+) (: {$($inner:tt)*} $($rest:tt)*) $copy:tt) => { $crate::data!(@map $map [$($key)+] ($crate::data!({$($inner)*})) $($rest)*); };
    (@map $map:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => { $crate::data!(@map $map [$($key)+] ($crate::data!($value)) , $($rest)*); };
    (@map $map:ident ($($key:tt)+) (: $value:expr) $copy:tt) => { $crate::data!(@map $map [$($key)+] ($crate::data!($value))); };
    (@map $map:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => { $crate::data!(@map $map ($key) (: $($rest)*) (: $($rest)*)); };
    (@map $map:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => { $crate::data!(@map $map ($($key)* $tt) ($($rest)*) ($($rest)*)); };

    (null) => { $crate::data::Data::Null };
    (true) => { $crate::data::Data::Bool(true) };
    (false) => { $crate::data::Data::Bool(false) };
    ([]) => { $crate::data::Data::List(vec![]) };
    ([ $($tt:tt)+ ]) => { $crate::data::Data::List($crate::data!(@list [] $($tt)+)) };
    ({}) => { $crate::data::Data::Map($crate::data::Map::new()) };
    ({ $($tt:tt)+ }) => {
        $crate::data::Data::Map({
            let mut map = $crate::data::Map::new();
            $crate::data!(@map map () ($($tt)+) ($($tt)+));
            map
        })
    };
    ($other:expr) => { $crate::data::to_data(&$other).expect("a value grafta builds is data") };
}


