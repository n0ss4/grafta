#![deny(clippy::all)]
// The decoder keys fields by the names the file's schema gives them.
#![allow(clippy::disallowed_methods)]
#![allow(clippy::missing_errors_doc)]

use thiserror::Error;

#[derive(Debug, Error)]
pub enum KiwiError {
    #[error("unexpected end of input")]
    UnexpectedEof,
    #[error("invalid UTF-8 string")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("invalid schema: {0}")]
    InvalidSchema(String),
    #[error("unknown definition: {0}")]
    UnknownDefinition(String),
    #[error("unknown field tag {tag} in message {msg}")]
    UnknownTag { tag: u32, msg: String },
    #[error("unknown enum value {value} in {def}")]
    UnknownEnumValue { value: u32, def: String },
    #[error("values nest deeper than {limit} levels, the most grafta reads")]
    TooDeep { limit: usize },
    #[error("the message holds more than {limit} values for its size, the most grafta reads")]
    TooManyValues { limit: u64 },

    // ── Encoding ─────────────────────────────────────────────────────────
    //
    // A `Value` that does not fit its definition is a bug in whatever built
    // it, not bad input, so every variant below names the definition and the
    // field a reader would have to go and look at.
    #[error("{def}.{field}: expected {expected}, found {found}")]
    TypeMismatch { def: String, field: String, expected: &'static str, found: &'static str },
    #[error("{def}.{field} is missing")]
    MissingField { def: String, field: String },
    #[error("{def} has no field named {field}")]
    UnknownFieldName { def: String, field: String },
    #[error("unknown enum name {name} in {def}")]
    UnknownEnumName { def: String, name: String },
    #[error("expected an object for {def}, found {found}")]
    NotAnObject { def: String, found: &'static str },
    #[error("{def}.{field}: {value} does not fit in a byte")]
    ByteOutOfRange { def: String, field: String, value: u32 },
}

// ── ByteReader ────────────────────────────────────────────────────────────────

pub struct ByteReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn read_byte(&mut self) -> Result<u8, KiwiError> {
        if self.pos >= self.data.len() {
            return Err(KiwiError::UnexpectedEof);
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    pub fn read_var_uint(&mut self) -> Result<u32, KiwiError> {
        let mut value: u32 = 0;
        let mut shift = 0u32;
        loop {
            let b = self.read_byte()? as u32;
            value |= (b & 0x7F) << shift;
            if b < 0x80 {
                return Ok(value);
            }
            shift += 7;
            if shift >= 35 {
                return Err(KiwiError::InvalidSchema("varuint overflow".into()));
            }
        }
    }

    pub fn read_var_int(&mut self) -> Result<i32, KiwiError> {
        // The shift must happen on the unsigned value. Casting to i32 first
        // makes `>>` arithmetic, which drags the sign bit down and mis-decodes
        // every value of magnitude 2^30 or more — `1073741824` came back as
        // `-1073741824`, and `i32::MIN` as `0`. `read_var_int64` never had the
        // bug because it shifts a u64. Found by writing the encoder: a value
        // decoded wrongly is a value written back wrongly, silently.
        let v = self.read_var_uint()?;
        Ok(if v & 1 != 0 { !(v >> 1) as i32 } else { (v >> 1) as i32 })
    }

    pub fn read_var_uint64(&mut self) -> Result<u64, KiwiError> {
        let mut value: u64 = 0;
        let mut shift = 0u32;
        loop {
            let b = self.read_byte()? as u64;
            if shift < 56 {
                value |= (b & 0x7F) << shift;
                if b < 0x80 {
                    return Ok(value);
                }
                shift += 7;
            } else {
                // last byte — no mask
                value |= b << shift;
                return Ok(value);
            }
        }
    }

    pub fn read_var_int64(&mut self) -> Result<i64, KiwiError> {
        let v = self.read_var_uint64()?;
        Ok(if v & 1 != 0 {
            !(v >> 1) as i64
        } else {
            (v >> 1) as i64
        })
    }

    pub fn read_var_float(&mut self) -> Result<f32, KiwiError> {
        let first = self.read_byte()?;
        if first == 0 {
            return Ok(0.0);
        }
        if self.pos + 3 > self.data.len() {
            return Err(KiwiError::UnexpectedEof);
        }
        let b1 = self.data[self.pos] as u32;
        let b2 = self.data[self.pos + 1] as u32;
        let b3 = self.data[self.pos + 2] as u32;
        self.pos += 3;
        let bits = (first as u32) | (b1 << 8) | (b2 << 16) | (b3 << 24);
        // inverse of write: rotate left by 23 = (bits << 23) | (bits >> 9)
        let bits = bits.rotate_left(23);
        Ok(f32::from_bits(bits))
    }

    pub fn read_string(&mut self) -> Result<String, KiwiError> {
        let start = self.pos;
        while self.pos < self.data.len() && self.data[self.pos] != 0 {
            self.pos += 1;
        }
        if self.pos >= self.data.len() {
            return Err(KiwiError::UnexpectedEof);
        }
        let s = String::from_utf8(self.data[start..self.pos].to_vec())?;
        self.pos += 1; // skip null terminator
        Ok(s)
    }

    /// The bytes not read yet, which is the most elements a length read from
    /// the input can be trusted for.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn read_byte_array(&mut self) -> Result<Vec<u8>, KiwiError> {
        let len = self.read_var_uint()? as usize;
        if self.pos + len > self.data.len() {
            return Err(KiwiError::UnexpectedEof);
        }
        let bytes = self.data[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Ok(bytes)
    }
}

// ── ByteWriter ────────────────────────────────────────────────────────────────

/// The mirror of [`ByteReader`], one method per read. Every method satisfies
/// the same law: write a value, read it back, get the value.
#[derive(Debug, Default)]
pub struct ByteWriter {
    data: Vec<u8>,
}

impl ByteWriter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn write_byte(&mut self, value: u8) {
        self.data.push(value);
    }

    pub fn write_var_uint(&mut self, mut value: u32) {
        loop {
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                self.data.push(byte);
                return;
            }
            self.data.push(byte | 0x80);
        }
    }

    /// Zigzag, so that small negative numbers stay small.
    pub fn write_var_int(&mut self, value: i32) {
        let zigzag = if value < 0 { ((!value as u32) << 1) | 1 } else { (value as u32) << 1 };
        self.write_var_uint(zigzag);
    }

    pub fn write_var_uint64(&mut self, mut value: u64) {
        // The reader stops masking at shift 56 and takes the eighth byte
        // whole, so the writer must hand it one there rather than continue
        // in seven-bit groups.
        for _ in 0..8 {
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                self.data.push(byte);
                return;
            }
            self.data.push(byte | 0x80);
        }
        self.data.push(value as u8);
    }

    pub fn write_var_int64(&mut self, value: i64) {
        let zigzag = if value < 0 { ((!value as u64) << 1) | 1 } else { (value as u64) << 1 };
        self.write_var_uint64(zigzag);
    }

    /// Four bytes, rotated so the exponent leads — except for zero and
    /// denormals, which the format stores as a single zero byte. That is not
    /// this writer's choice: the reader special-cases a leading zero, so
    /// anything else would not read back. See research D7.
    pub fn write_var_float(&mut self, value: f32) {
        let bits = value.to_bits();
        // An exponent of zero means zero or a denormal, and the low byte of
        // the rotated word is exactly the exponent.
        if bits & 0x7F80_0000 == 0 {
            self.data.push(0);
            return;
        }
        self.data.extend_from_slice(&bits.rotate_right(23).to_le_bytes());
    }

    /// UTF-8 and a null terminator, which is why a Kiwi string cannot hold an
    /// interior null. The decoder cannot produce one, so a value that came
    /// from a file cannot carry one either.
    pub fn write_string(&mut self, value: &str) {
        self.data.extend_from_slice(value.as_bytes());
        self.data.push(0);
    }

    pub fn write_byte_array(&mut self, value: &[u8]) {
        self.write_var_uint(value.len() as u32);
        self.data.extend_from_slice(value);
    }
}

// ── Schema types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionKind {
    Enum,
    Struct,
    Message,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRef {
    Bool,
    Byte,
    Int,
    Uint,
    Float,
    Str,
    Int64,
    Uint64,
    Named(String),
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub type_ref: Option<TypeRef>,
    pub is_array: bool,
    pub value: u32,
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub name: String,
    pub kind: DefinitionKind,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct Schema {
    pub definitions: Vec<Definition>,
}

impl Schema {
    pub fn find_definition(&self, name: &str) -> Option<&Definition> {
        self.definitions.iter().find(|d| d.name == name)
    }
}

// ── decode_binary_schema ──────────────────────────────────────────────────────

const PRIMITIVES: &[&str] = &["bool", "byte", "int", "uint", "float", "string", "int64", "uint64"];

pub fn decode_binary_schema(data: &[u8]) -> Result<Schema, KiwiError> {
    let mut r = ByteReader::new(data);
    let def_count = r.read_var_uint()? as usize;
    let mut definitions: Vec<Definition> = Vec::with_capacity(def_count.min(r.remaining()));

    // First pass: read raw definitions storing type indices
    struct RawField {
        name: String,
        type_raw: i32,
        is_array: bool,
        value: u32,
    }
    struct RawDef {
        name: String,
        kind: u8,
        fields: Vec<RawField>,
    }

    let mut raw_defs: Vec<RawDef> = Vec::with_capacity(def_count.min(r.remaining()));
    for _ in 0..def_count {
        let name = r.read_string()?;
        let kind = r.read_byte()?;
        let field_count = r.read_var_uint()? as usize;
        let mut fields = Vec::with_capacity(field_count.min(r.remaining()));
        for _ in 0..field_count {
            let field_name = r.read_string()?;
            let type_raw = r.read_var_int()?;
            let is_array = r.read_byte()? & 1 != 0;
            let value = r.read_var_uint()?;
            fields.push(RawField { name: field_name, type_raw, is_array, value });
        }
        raw_defs.push(RawDef { name, kind, fields });
    }

    // Second pass: bind type references
    for raw in &raw_defs {
        let kind = match raw.kind {
            0 => DefinitionKind::Enum,
            1 => DefinitionKind::Struct,
            2 => DefinitionKind::Message,
            k => return Err(KiwiError::InvalidSchema(format!("unknown kind {k}"))),
        };
        let mut fields = Vec::with_capacity(raw.fields.len());
        for rf in &raw.fields {
            let type_ref = if kind == DefinitionKind::Enum {
                None
            } else if rf.type_raw < 0 {
                let idx = (!(rf.type_raw)) as usize; // ~type_raw = -type_raw - 1
                if idx >= PRIMITIVES.len() {
                    return Err(KiwiError::InvalidSchema(format!("primitive index {idx} out of range")));
                }
                Some(match PRIMITIVES[idx] {
                    "bool" => TypeRef::Bool,
                    "byte" => TypeRef::Byte,
                    "int" => TypeRef::Int,
                    "uint" => TypeRef::Uint,
                    "float" => TypeRef::Float,
                    "string" => TypeRef::Str,
                    "int64" => TypeRef::Int64,
                    "uint64" => TypeRef::Uint64,
                    p => return Err(KiwiError::InvalidSchema(format!("unknown primitive {p}"))),
                })
            } else {
                let idx = rf.type_raw as usize;
                if idx >= raw_defs.len() {
                    return Err(KiwiError::InvalidSchema(format!("definition index {idx} out of range")));
                }
                Some(TypeRef::Named(raw_defs[idx].name.clone()))
            };
            fields.push(Field {
                name: rf.name.clone(),
                type_ref,
                is_array: rf.is_array,
                value: rf.value,
            });
        }
        definitions.push(Definition { name: raw.name.clone(), kind, fields });
    }

    Ok(Schema { definitions })
}

// ── Names ─────────────────────────────────────────────────────────────────────

/// One shared copy of a field or enum name, for the life of the process.
///
/// A decoded message repeats a few thousand schema words millions of times:
/// 23.6 million field-name strings on a large file. Interned,
/// each value stores a pointer. The set of names is bounded by the schemas
/// read, so what is kept is kilobytes.
pub fn intern(name: &str) -> &'static str {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};
    static NAMES: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let mut names = NAMES.get_or_init(|| Mutex::new(HashSet::new())).lock().unwrap_or_else(|e| e.into_inner());
    if let Some(known) = names.get(name) {
        return known;
    }
    let leaked: &'static str = Box::leak(name.to_owned().into_boxed_str());
    names.insert(leaked);
    leaked
}

/// An object's fields, in the order they were read or inserted.
///
/// A vector rather than a hash table: an object has tens of fields, order must
/// survive a round trip, and a table per object was most of an open file's
/// memory. The methods are the ones the workspace used on `IndexMap`.
#[derive(Clone, Default, PartialEq)]
pub struct Fields(Vec<(&'static str, Value)>);

impl Fields {
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    fn position(&self, key: &str) -> Option<usize> {
        self.0.iter().position(|(name, _)| std::ptr::eq(*name, key) || *name == key)
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.position(key).map(|at| &self.0[at].1)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.position(key).map(|at| &mut self.0[at].1)
    }

    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.position(key).is_some()
    }

    /// Set a field, in place when it exists, at the end when it does not.
    pub fn insert(&mut self, key: impl AsRef<str>, value: Value) -> Option<Value> {
        let key = key.as_ref();
        match self.position(key) {
            Some(at) => Some(std::mem::replace(&mut self.0[at].1, value)),
            None => {
                self.0.push((intern(key), value));
                None
            }
        }
    }

    /// The field, inserted first with `make` when it is missing.
    pub fn get_or_insert_with(&mut self, key: &str, make: impl FnOnce() -> Value) -> &mut Value {
        let at = match self.position(key) {
            Some(at) => at,
            None => {
                self.0.push((intern(key), make()));
                self.0.len() - 1
            }
        };
        &mut self.0[at].1
    }

    /// Remove a field, keeping the others in order.
    pub fn shift_remove(&mut self, key: &str) -> Option<Value> {
        self.position(key).map(|at| self.0.remove(at).1)
    }

    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.shift_remove(key)
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&'static str, &mut Value) -> bool) {
        self.0.retain_mut(|(name, value)| keep(name, value));
    }

    pub fn keys(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.0.iter().map(|(name, _)| *name)
    }

    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.0.iter().map(|(_, value)| value)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &Value)> {
        self.0.iter().map(|(name, value)| (*name, value))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&'static str, &mut Value)> {
        self.0.iter_mut().map(|(name, value)| (*name, value))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for Fields {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.0.iter().map(|(name, value)| (name, value))).finish()
    }
}

impl<K: AsRef<str>> FromIterator<(K, Value)> for Fields {
    fn from_iter<I: IntoIterator<Item = (K, Value)>>(items: I) -> Self {
        let mut fields = Self::new();
        fields.extend(items);
        fields
    }
}

impl<K: AsRef<str>> Extend<(K, Value)> for Fields {
    fn extend<I: IntoIterator<Item = (K, Value)>>(&mut self, items: I) {
        for (key, value) in items {
            self.insert(key, value);
        }
    }
}

impl<K: AsRef<str>, const N: usize> From<[(K, Value); N]> for Fields {
    fn from(items: [(K, Value); N]) -> Self {
        items.into_iter().collect()
    }
}

impl IntoIterator for Fields {
    type Item = (&'static str, Value);
    type IntoIter = std::vec::IntoIter<(&'static str, Value)>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Fields {
    type Item = (&'static str, &'a Value);
    type IntoIter = std::iter::Map<std::slice::Iter<'a, (&'static str, Value)>, fn(&'a (&'static str, Value)) -> (&'static str, &'a Value)>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().map(|(name, value)| (*name, value))
    }
}

impl std::ops::Index<&str> for Fields {
    type Output = Value;
    fn index(&self, key: &str) -> &Value {
        self.get(key).unwrap_or_else(|| panic!("no field {key}"))
    }
}

// ── Value ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i32),
    Uint(u32),
    Int64(i64),
    Uint64(u64),
    Float(f32),
    Str(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Object(Fields),
    Enum(&'static str),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(m) => m.get(key),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s.as_str()),
            Value::Enum(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Value::Uint(v) => Some(*v),
            Value::Int(v) => Some(*v as u32),
            _ => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Value::Float(v) => Some(*v),
            Value::Int(v) => Some(*v as f32),
            Value::Uint(v) => Some(*v as f32),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(v) => Some(*v),
            _ => None,
        }
    }
}

// ── decode_message ────────────────────────────────────────────────────────────

pub fn decode_message(root_name: &str, data: &[u8], schema: &Schema) -> Result<Value, KiwiError> {
    decode_message_within(root_name, data, schema, MAX_VALUES)
}

/// [`decode_message`], holding at most `most_values`, for a caller that knows
/// more of what the message came in than the message says.
pub fn decode_message_within(root_name: &str, data: &[u8], schema: &Schema, most_values: u64) -> Result<Value, KiwiError> {
    let def = schema
        .find_definition(root_name)
        .ok_or_else(|| KiwiError::UnknownDefinition(root_name.to_string()))?;
    let mut reader = ByteReader::new(data);
    let mut budget = Budget::for_message(data.len(), most_values);
    decode_value(&mut reader, def, schema, &mut budget)
}

/// How far a message may nest. Measured on 13 real files: at most 24.
const MAX_DEPTH: usize = 64;
/// The most values any message may hold, about 8 GiB of them. Measured on 13
/// real files: at most 70,109,169.
const MAX_VALUES: u64 = 1 << 28;

/// What a message may still spend. A struct reads no input, so what it holds has
/// to be counted: a schema alone can nest without end, or double at each level.
struct Budget {
    depth: usize,
    values: u64,
    limit: u64,
}

impl Budget {
    /// At most a value per two bytes, and a few for a small message. Measured on
    /// 13 real files, and on thousands of copies of their frames: at most 0.32
    /// per byte. An empty struct is a value in one byte, and takes 32 to hold.
    fn for_message(bytes: usize, most: u64) -> Self {
        let limit = (bytes as u64 / 2).saturating_add(1024).min(MAX_VALUES).min(most);
        Self { depth: 0, values: 0, limit }
    }

    fn value(&mut self) -> Result<(), KiwiError> {
        self.values += 1;
        if self.values > self.limit {
            return Err(KiwiError::TooManyValues { limit: self.limit });
        }
        Ok(())
    }
}

fn decode_value(reader: &mut ByteReader<'_>, def: &Definition, schema: &Schema, budget: &mut Budget) -> Result<Value, KiwiError> {
    if budget.depth == MAX_DEPTH {
        return Err(KiwiError::TooDeep { limit: MAX_DEPTH });
    }
    budget.depth += 1;
    let value = decode_definition(reader, def, schema, budget);
    budget.depth -= 1;
    value
}

fn decode_definition(reader: &mut ByteReader<'_>, def: &Definition, schema: &Schema, budget: &mut Budget) -> Result<Value, KiwiError> {
    match def.kind {
        DefinitionKind::Enum => {
            let n = reader.read_var_uint()?;
            let field = def.fields.iter().find(|f| f.value == n).ok_or_else(|| {
                KiwiError::UnknownEnumValue { value: n, def: def.name.clone() }
            })?;
            Ok(Value::Enum(intern(&field.name)))
        }
        DefinitionKind::Struct => {
            let mut map = Fields::with_capacity(def.fields.len());
            for field in &def.fields {
                let v = read_field_value(reader, field, schema, budget)?;
                map.0.push((intern(&field.name), v));
            }
            Ok(Value::Object(map))
        }
        DefinitionKind::Message => {
            let mut map = Fields::new();
            loop {
                let tag = reader.read_var_uint()?;
                if tag == 0 {
                    break;
                }
                if let Some(field) = def.fields.iter().find(|f| f.value == tag) {
                    let v = read_field_value(reader, field, schema, budget)?;
                    map.insert(intern(&field.name), v);
                } else {
                    // Unknown field - skip is not straightforward without type info; error
                    return Err(KiwiError::UnknownTag { tag, msg: def.name.clone() });
                }
            }
            Ok(Value::Object(map))
        }
    }
}

fn read_field_value(reader: &mut ByteReader<'_>, field: &Field, schema: &Schema, budget: &mut Budget) -> Result<Value, KiwiError> {
    match &field.type_ref {
        None => {
            // ENUM field without type ref shouldn't appear in struct/message; return null
            Ok(Value::Null)
        }
        Some(type_ref) => {
            if field.is_array {
                if *type_ref == TypeRef::Byte {
                    return Ok(Value::Bytes(reader.read_byte_array()?));
                }
                let len = reader.read_var_uint()? as usize;
                let mut values = Vec::with_capacity(len.min(reader.remaining()));
                for _ in 0..len {
                    values.push(read_single_value(reader, type_ref, schema, budget)?);
                }
                Ok(Value::Array(values))
            } else {
                read_single_value(reader, type_ref, schema, budget)
            }
        }
    }
}

fn read_single_value(reader: &mut ByteReader<'_>, type_ref: &TypeRef, schema: &Schema, budget: &mut Budget) -> Result<Value, KiwiError> {
    budget.value()?;
    match type_ref {
        TypeRef::Bool => Ok(Value::Bool(reader.read_byte()? != 0)),
        TypeRef::Byte => Ok(Value::Uint(reader.read_byte()? as u32)),
        TypeRef::Int => Ok(Value::Int(reader.read_var_int()?)),
        TypeRef::Uint => Ok(Value::Uint(reader.read_var_uint()?)),
        TypeRef::Float => Ok(Value::Float(reader.read_var_float()?)),
        TypeRef::Str => Ok(Value::Str(reader.read_string()?)),
        TypeRef::Int64 => Ok(Value::Int64(reader.read_var_int64()?)),
        TypeRef::Uint64 => Ok(Value::Uint64(reader.read_var_uint64()?)),
        TypeRef::Named(name) => {
            let def = schema
                .find_definition(name)
                .ok_or_else(|| KiwiError::UnknownDefinition(name.clone()))?;
            decode_value(reader, def, schema, budget)
        }
    }
}

// ── encode_message ────────────────────────────────────────────────────────────

/// Encode a [`Value`] against the schema it was decoded from.
///
/// The walk is driven by the `Definition`, never by the `Value` — research D3.
/// That is not a stylistic preference: `Value` does not carry enough to write
/// itself. A `TypeRef::Byte` decodes to `Value::Uint`, so a `Value::Uint` alone
/// cannot say whether to emit one raw byte or a varint; a `Value::Enum` holds a
/// *name*, and only the definition knows its number. The schema disambiguated
/// both on the way in, so the schema must drive the way out.
///
/// **Round-trip law**: for any `Value` [`decode_message`] produced from a
/// schema, `decode_message(encode_message(v)) == v`.
pub fn encode_message(root_name: &str, value: &Value, schema: &Schema) -> Result<Vec<u8>, KiwiError> {
    let def = schema
        .find_definition(root_name)
        .ok_or_else(|| KiwiError::UnknownDefinition(root_name.to_string()))?;
    let mut writer = ByteWriter::new();
    encode_value(&mut writer, value, def, schema)?;
    Ok(writer.into_bytes())
}

/// What a `Value` is, for an error message.
fn kind_of(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a bool",
        Value::Int(_) => "an int",
        Value::Uint(_) => "a uint",
        Value::Int64(_) => "an int64",
        Value::Uint64(_) => "a uint64",
        Value::Float(_) => "a float",
        Value::Str(_) => "a string",
        Value::Bytes(_) => "a byte array",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
        Value::Enum(_) => "an enum",
    }
}

fn encode_value(
    writer: &mut ByteWriter,
    value: &Value,
    def: &Definition,
    schema: &Schema,
) -> Result<(), KiwiError> {
    match def.kind {
        DefinitionKind::Enum => {
            let Value::Enum(name) = value else {
                return Err(KiwiError::NotAnObject { def: def.name.clone(), found: kind_of(value) });
            };
            // The name is what was decoded; the number is what goes on the
            // wire, and only the definition knows it.
            let field = def.fields.iter().find(|f| f.name == *name).ok_or_else(|| {
                KiwiError::UnknownEnumName { def: def.name.clone(), name: (*name).to_string() }
            })?;
            writer.write_var_uint(field.value);
            Ok(())
        }
        DefinitionKind::Struct => {
            let Value::Object(map) = value else {
                return Err(KiwiError::NotAnObject { def: def.name.clone(), found: kind_of(value) });
            };
            // Schema declaration order, every field, every time — the decoder
            // reads them positionally, so there is nothing to say a field was
            // skipped.
            for field in &def.fields {
                let field_value = map.get(&field.name).ok_or_else(|| KiwiError::MissingField {
                    def: def.name.clone(),
                    field: field.name.clone(),
                })?;
                encode_field_value(writer, field_value, field, def, schema)?;
            }
            Ok(())
        }
        DefinitionKind::Message => {
            let Value::Object(map) = value else {
                return Err(KiwiError::NotAnObject { def: def.name.clone(), found: kind_of(value) });
            };
            // The map's order is the order the file wrote its tags (D4), so
            // iterating it reproduces the original tag sequence.
            for (name, field_value) in map {
                let field = def.fields.iter().find(|f| f.name == name).ok_or_else(|| {
                    KiwiError::UnknownFieldName { def: def.name.clone(), field: (*name).to_string() }
                })?;
                writer.write_var_uint(field.value);
                encode_field_value(writer, field_value, field, def, schema)?;
            }
            writer.write_var_uint(0); // the terminator the decoder stops on
            Ok(())
        }
    }
}

fn encode_field_value(
    writer: &mut ByteWriter,
    value: &Value,
    field: &Field,
    def: &Definition,
    schema: &Schema,
) -> Result<(), KiwiError> {
    let Some(type_ref) = &field.type_ref else {
        // An enum's own fields carry no type and the decoder consumes nothing
        // for them, so neither does the encoder.
        return Ok(());
    };

    if field.is_array {
        // A byte array is length-prefixed bytes, not an array of bytes.
        if *type_ref == TypeRef::Byte {
            let Value::Bytes(bytes) = value else {
                return Err(KiwiError::TypeMismatch {
                    def: def.name.clone(),
                    field: field.name.clone(),
                    expected: "a byte array",
                    found: kind_of(value),
                });
            };
            writer.write_byte_array(bytes);
            return Ok(());
        }

        let Value::Array(values) = value else {
            return Err(KiwiError::TypeMismatch {
                def: def.name.clone(),
                field: field.name.clone(),
                expected: "an array",
                found: kind_of(value),
            });
        };
        writer.write_var_uint(values.len() as u32);
        for item in values {
            encode_single_value(writer, item, type_ref, field, def, schema)?;
        }
        return Ok(());
    }

    encode_single_value(writer, value, type_ref, field, def, schema)
}

fn encode_single_value(
    writer: &mut ByteWriter,
    value: &Value,
    type_ref: &TypeRef,
    field: &Field,
    def: &Definition,
    schema: &Schema,
) -> Result<(), KiwiError> {
    let mismatch = |expected: &'static str| KiwiError::TypeMismatch {
        def: def.name.clone(),
        field: field.name.clone(),
        expected,
        found: kind_of(value),
    };

    match type_ref {
        TypeRef::Bool => match value {
            Value::Bool(v) => writer.write_byte(u8::from(*v)),
            _ => return Err(mismatch("a bool")),
        },
        // One raw byte, though it decoded into a `Value::Uint`. This is the
        // ambiguity D3 exists to prevent: written as a varint it would be
        // right for 0..=127 and wrong above.
        TypeRef::Byte => match value {
            Value::Uint(v) => {
                let byte = u8::try_from(*v).map_err(|_| KiwiError::ByteOutOfRange {
                    def: def.name.clone(),
                    field: field.name.clone(),
                    value: *v,
                })?;
                writer.write_byte(byte);
            }
            _ => return Err(mismatch("a uint holding a byte")),
        },
        TypeRef::Int => match value {
            Value::Int(v) => writer.write_var_int(*v),
            _ => return Err(mismatch("an int")),
        },
        TypeRef::Uint => match value {
            Value::Uint(v) => writer.write_var_uint(*v),
            _ => return Err(mismatch("a uint")),
        },
        TypeRef::Float => match value {
            Value::Float(v) => writer.write_var_float(*v),
            _ => return Err(mismatch("a float")),
        },
        TypeRef::Str => match value {
            Value::Str(v) => writer.write_string(v),
            _ => return Err(mismatch("a string")),
        },
        TypeRef::Int64 => match value {
            Value::Int64(v) => writer.write_var_int64(*v),
            _ => return Err(mismatch("an int64")),
        },
        TypeRef::Uint64 => match value {
            Value::Uint64(v) => writer.write_var_uint64(*v),
            _ => return Err(mismatch("a uint64")),
        },
        TypeRef::Named(name) => {
            let nested = schema
                .find_definition(name)
                .ok_or_else(|| KiwiError::UnknownDefinition(name.clone()))?;
            encode_value(writer, value, nested, schema)?;
        }
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
