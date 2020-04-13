use crate::{Error, ErrorKind};
#[cfg(feature = "graphson2")]
use gremlin_client::{GValue, ToGValue, VertexProperty};
use log::trace;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

#[derive(Clone, Debug)]
pub enum Value {
    Array(Vec<Value>),
    Bool(bool),
    Float64(f64),
    Int64(i64),
    Map(HashMap<String, Value>),
    Null,
    String(String),
    UInt64(u64),
}

impl From<HashMap<String, Value>> for Value {
    fn from(map: HashMap<String, Value>) -> Self {
        Value::Map(map)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Array(a), Value::Array(oa)) => a == oa,
            (Value::Bool(b), Value::Bool(ob)) => b == ob,
            (Value::Float64(f), Value::Float64(of)) => f == of,
            (Value::Int64(i), Value::Int64(oi)) => i == oi,
            (Value::Map(m), Value::Map(om)) => m == om,
            (Value::Null, Value::Null) => true,
            (Value::String(s), Value::String(os)) => s == os,
            (Value::UInt64(i), Value::UInt64(oi)) => i == oi,
            (_, _) => false,
        }
    }
}

#[cfg(feature = "graphson2")]
impl ToGValue for Value {
    fn to_gvalue(&self) -> GValue {
        match self {
            Value::Array(a) => {
                let mut v = Vec::new();
                for val in a {
                    v.push(val.to_gvalue());
                }
                GValue::List(gremlin_client::List::new(v))
            }
            Value::Bool(b) => b.to_gvalue(),
            Value::Float64(f) => f.to_gvalue(),
            Value::Int64(i) => i.to_gvalue(),
            Value::Map(hm) => {
                let mut m = HashMap::new();
                for (k, v) in hm.iter() {
                    m.insert(k.to_string(), v.to_gvalue());
                }
                GValue::Map(m.into())
            }
            Value::Null => GValue::String("".to_string()),
            Value::String(s) => s.to_gvalue(),
            // Note, the conversion of a UInt64 to an Int64 may be lossy, but GValue has
            // neither unsigned integer types, nor a try/error interface for value conversion
            Value::UInt64(i) => GValue::Int64(*i as i64),
        }
    }
}

impl TryFrom<GValue> for Value {
    type Error = Error;

    fn try_from(gvalue: GValue) -> Result<Value, Error> {
        match gvalue {
            GValue::Vertex(_v) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::Vertex".to_string()),
                None,
            )),
            GValue::Edge(_e) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::Edge".to_string()),
                None,
            )),
            GValue::VertexProperty(vp) => Ok(vp.try_into()?),
            GValue::Property(_p) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::Property".to_string()),
                None,
            )),
            GValue::Uuid(u) => Ok(Value::String(u.to_hyphenated().to_string())),
            GValue::Int32(i) => Ok(Value::Int64(i as i64)),
            GValue::Int64(i) => Ok(Value::Int64(i)),
            GValue::Float(f) => Ok(Value::Float64(f as f64)),
            GValue::Double(f) => Ok(Value::Float64(f)),
            GValue::Date(_d) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::Date".to_string()),
                None,
            )),
            GValue::List(_l) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::List".to_string()),
                None,
            )),
            GValue::Set(_s) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::Set".to_string()),
                None,
            )),
            GValue::Map(_m) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::Map".to_string()),
                None,
            )),
            GValue::Token(_t) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::Token".to_string()),
                None,
            )),
            GValue::String(s) => Ok(Value::String(s)),
            GValue::Path(_p) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::Path".to_string()),
                None,
            )),
            GValue::TraversalMetrics(_tm) => Err(Error::new(
                ErrorKind::TypeConversionFailed(
                    "Could not convert GValue::TraversalMetrics".to_string(),
                ),
                None,
            )),
            GValue::Metric(_m) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::Metric".to_string()),
                None,
            )),
            GValue::TraversalExplanation(_m) => Err(Error::new(
                ErrorKind::TypeConversionFailed(
                    "Could not convert GValue::TraversalExplanation".to_string(),
                ),
                None,
            )),
            GValue::IntermediateRepr(_ir) => Err(Error::new(
                ErrorKind::TypeConversionFailed(
                    "Could not convert GValue::IntermediateRepr".to_string(),
                ),
                None,
            )),
            GValue::P(_p) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::P".to_string()),
                None,
            )),
            GValue::T(_t) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::T".to_string()),
                None,
            )),
            GValue::Bytecode(_bc) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::Bytecode".to_string()),
                None,
            )),
            GValue::Traverser(_t) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::Traverser".to_string()),
                None,
            )),
            GValue::Scope(_s) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::Scope".to_string()),
                None,
            )),
            GValue::Order(_o) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::Order".to_string()),
                None,
            )),
            GValue::Bool(b) => Ok(Value::Bool(b)),
            GValue::TextP(_tp) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::TextP".to_string()),
                None,
            )),
            GValue::Pop(_p) => Err(Error::new(
                ErrorKind::TypeConversionFailed("Could not convert GValue::Pop".to_string()),
                None,
            )),
            GValue::Cardinality(_c) => Err(Error::new(
                ErrorKind::TypeConversionFailed(
                    "Could not convert GValue::Cardinality".to_string(),
                ),
                None,
            )),
        }
    }
}

impl TryFrom<serde_json::Value> for Value {
    type Error = Error;

    fn try_from(value: serde_json::Value) -> Result<Value, Error> {
        match value {
            serde_json::Value::Array(a) => {
                let mut v = Vec::new();
                for val in a {
                    v.push(val.try_into()?);
                }
                Ok(Value::Array(v))
            }
            serde_json::Value::Bool(b) => Ok(Value::Bool(b)),
            serde_json::Value::Null => Ok(Value::Null),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Value::Int64(i))
                } else if let Some(i) = n.as_u64() {
                    Ok(Value::UInt64(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(Value::Float64(f))
                } else {
                    Err(Error::new(
                        ErrorKind::TypeConversionFailed(
                            "Expected a serde_json Number to be an i64, u64, or f64.".to_string(),
                        ),
                        None,
                    ))
                }
            }
            serde_json::Value::String(s) => Ok(Value::String(s)),
            serde_json::Value::Object(m) => {
                let mut hm = HashMap::new();
                for (k, v) in m.into_iter() {
                    hm.insert(k, v.try_into()?);
                }
                Ok(Value::Map(hm))
            }
        }
    }
}

impl TryFrom<VertexProperty> for Value {
    type Error = Error;

    fn try_from(vp: VertexProperty) -> Result<Value, Error> {
        trace!("Value::try_from vp: {:#?}", vp);

        Ok(vp
            .take::<GValue>()
            .map_err(|_e| {
                Error::new(
                    ErrorKind::TypeConversionFailed("VertexProperty".to_string()),
                    None,
                )
            })?
            .try_into()?)
    }
}

impl TryFrom<Value> for bool {
    type Error = Error;

    fn try_from(value: Value) -> Result<bool, Self::Error> {
        if let Value::Bool(b) = value {
            Ok(b)
        } else {
            Err(Error::new(
                ErrorKind::TypeConversionFailed("Expected a boolean.".to_string()),
                None,
            ))
        }
    }
}

impl TryFrom<Value> for Vec<bool> {
    type Error = Error;

    fn try_from(value: Value) -> Result<Vec<bool>, Self::Error> {
        let mut v = Vec::new();
        if let Value::Array(a) = value {
            for val in a {
                v.push(val.try_into()?)
            }
            Ok(v)
        } else {
            Err(Error::new(
                ErrorKind::TypeConversionFailed("Expected a Value::Array.".to_string()),
                None,
            ))
        }
    }
}

impl TryFrom<Value> for f64 {
    type Error = Error;

    fn try_from(value: Value) -> Result<f64, Self::Error> {
        if let Value::Float64(f) = value {
            Ok(f)
        } else {
            Err(Error::new(
                ErrorKind::TypeConversionFailed("Expected an f64.".to_string()),
                None,
            ))
        }
    }
}

impl TryFrom<Value> for Vec<f64> {
    type Error = Error;

    fn try_from(value: Value) -> Result<Vec<f64>, Self::Error> {
        let mut v = Vec::new();
        if let Value::Array(a) = value {
            for val in a {
                v.push(val.try_into()?)
            }
            Ok(v)
        } else {
            Err(Error::new(
                ErrorKind::TypeConversionFailed("Expected a Value::Array.".to_string()),
                None,
            ))
        }
    }
}

impl TryFrom<Value> for i32 {
    type Error = Error;

    fn try_from(value: Value) -> Result<i32, Self::Error> {
        match value {
            Value::Int64(i) => {
                if i >= (i32::min_value() as i64) && i <= (i32::max_value() as i64) {
                    Ok(i as i32)
                } else {
                    Err(Error::new(
                        ErrorKind::TypeConversionFailed(
                            "Expected an i64 or u64 within the i32 range.".to_string(),
                        ),
                        None,
                    ))
                }
            }
            Value::UInt64(i) => {
                if i <= (i32::max_value() as u64) {
                    Ok(i as i32)
                } else {
                    Err(Error::new(
                        ErrorKind::TypeConversionFailed(
                            "Expected an i64 or u64 within the i32 range.".to_string(),
                        ),
                        None,
                    ))
                }
            }
            _ => Err(Error::new(
                ErrorKind::TypeConversionFailed(
                    "Expected an i64 or u64 within the i32 range.".to_string(),
                ),
                None,
            )),
        }
    }
}

impl TryFrom<Value> for Vec<i32> {
    type Error = Error;

    fn try_from(value: Value) -> Result<Vec<i32>, Self::Error> {
        let mut v = Vec::new();
        if let Value::Array(a) = value {
            for val in a {
                v.push(val.try_into()?)
            }
            Ok(v)
        } else {
            Err(Error::new(
                ErrorKind::TypeConversionFailed("Expected a Value::Array.".to_string()),
                None,
            ))
        }
    }
}

impl TryFrom<Value> for String {
    type Error = Error;

    fn try_from(value: Value) -> Result<String, Self::Error> {
        if let Value::String(s) = value {
            Ok(s)
        } else {
            Err(Error::new(
                ErrorKind::TypeConversionFailed("Expected an String.".to_string()),
                None,
            ))
        }
    }
}

impl TryFrom<Value> for Vec<String> {
    type Error = Error;

    fn try_from(value: Value) -> Result<Vec<String>, Self::Error> {
        let mut v = Vec::new();
        if let Value::Array(a) = value {
            match a.get(0) {
                Some(Value::Null) => (), // If the array composed of null values, return an empty vector, indicating null to Juniper.
                _ => {
                    // If the array has anything other than a null, try to do the conversation to a String Vector.
                    for val in a {
                        v.push(val.try_into()?)
                    }
                }
            }
            Ok(v)
        } else {
            Err(Error::new(
                ErrorKind::TypeConversionFailed("Expected a Value::Array.".to_string()),
                None,
            ))
        }
    }
}

impl TryFrom<Value> for serde_json::Value {
    type Error = Error;

    fn try_from(value: Value) -> Result<serde_json::Value, Error> {
        match value {
            Value::Array(a) => {
                let mut v = Vec::new();
                for val in a {
                    v.push(val.try_into()?)
                }
                Ok(serde_json::Value::Array(v))
            }
            Value::Bool(b) => Ok(serde_json::Value::Bool(b)),
            Value::Float64(f) => Ok(serde_json::Value::Number(
                serde_json::Number::from_f64(f).ok_or_else(|| {
                    Error::new(
                        ErrorKind::TypeConversionFailed(
                            "Expected f64 not to be infinite or NaN.".into(),
                        ),
                        None,
                    )
                })?,
            )),
            Value::Int64(i) => Ok(serde_json::Value::Number(i.into())),
            Value::Map(hm) => {
                let mut m = serde_json::Map::new();
                for (k, v) in hm.into_iter() {
                    m.insert(k.to_string(), v.try_into()?);
                }
                Ok(serde_json::Value::Object(m))
            }
            Value::Null => Ok(serde_json::Value::Null),
            Value::String(s) => Ok(serde_json::Value::String(s)),
            Value::UInt64(i) => Ok(serde_json::Value::Number(i.into())),
        }
    }
}
