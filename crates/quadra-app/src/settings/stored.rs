//! What a setting can hold (specs/15, "Settings"): a value, and what it is in JSON.

use serde_json::Value;

/// A value a setting can hold: what it is in JSON, and back. A value of another shape in
/// the file is no value: the setting is its default.
pub trait Stored: Clone + PartialEq + 'static {
    fn to_json(&self) -> Value;
    fn from_json(value: &Value) -> Option<Self>;
}

macro_rules! stored_numbers {
    ($($t:ty: $as:ident),*) => {$(
        impl Stored for $t {
            fn to_json(&self) -> Value {
                Value::from(*self)
            }
            fn from_json(value: &Value) -> Option<Self> {
                value.$as().and_then(|v| <$t>::try_from(v).ok())
            }
        }
    )*};
}

stored_numbers!(i32: as_i64, i64: as_i64, u8: as_u64, u32: as_u64, u64: as_u64, usize: as_u64);

impl Stored for f64 {
    fn to_json(&self) -> Value {
        Value::from(*self)
    }
    fn from_json(value: &Value) -> Option<Self> {
        value.as_f64()
    }
}

impl Stored for f32 {
    fn to_json(&self) -> Value {
        Value::from(*self as f64)
    }
    fn from_json(value: &Value) -> Option<Self> {
        value.as_f64().map(|v| v as f32)
    }
}

impl Stored for bool {
    fn to_json(&self) -> Value {
        Value::Bool(*self)
    }
    fn from_json(value: &Value) -> Option<Self> {
        value.as_bool()
    }
}

impl Stored for String {
    fn to_json(&self) -> Value {
        Value::String(self.clone())
    }
    fn from_json(value: &Value) -> Option<Self> {
        value.as_str().map(str::to_string)
    }
}

impl<T: Stored> Stored for Option<T> {
    fn to_json(&self) -> Value {
        self.as_ref().map_or(Value::Null, T::to_json)
    }
    fn from_json(value: &Value) -> Option<Self> {
        if value.is_null() { Some(None) } else { T::from_json(value).map(Some) }
    }
}

impl<T: Stored> Stored for Vec<T> {
    fn to_json(&self) -> Value {
        Value::Array(self.iter().map(T::to_json).collect())
    }
    fn from_json(value: &Value) -> Option<Self> {
        value.as_array()?.iter().map(T::from_json).collect()
    }
}
