use crate::Error;
use juniper::{DefaultScalarValue, FromInputValue, InputValue};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

/// Intermediate data structure for serialized values, allowing for translation between the values
/// returned by the back-end database (serde_json for Neo4j, and a library-specific seralized
/// format for Cosmos DB), and the serde_json format used to return data to the client.
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::engine::value::Value;
///
/// let v = Value::Bool(true);
/// ```
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

impl Value {

    /// Returns [`Value`] as a [`String`] or returns an error.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] variant [`TypeConversionFailed`] if the enum variant
    /// is not `String`. 
    ///
    /// # Example
    ///
    /// ```rust
    /// use warpgrapher::engine::value::Value;
    ///
    /// let v = Value::String("Joe".to_string());
    /// assert!(v.as_string().is_ok());
    /// ```
    pub fn as_string(&self) -> Result<String, Error> {
        match self {
            Value::String(s) => { Ok(s.to_string()) },
            _ => { Err(Error::TypeConversionFailed { src: "Value".to_string(), dst: "String".to_string() })}
        }
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float64(v)
    }
}

impl From<HashMap<String, Value>> for Value {
    fn from(map: HashMap<String, Value>) -> Self {
        Value::Map(map)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int64(v)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<u64> for Value {
    fn from(v: u64) -> Self {
        Value::UInt64(v)
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Value::Array(v)
    }
}

impl FromInputValue for Value {
    fn from_input_value(v: &InputValue) -> Option<Self> {
        if let InputValue::Scalar(scalar) = v {
            Some(match scalar {
                DefaultScalarValue::Int(i) => Value::Int64(i64::from(*i)),
                DefaultScalarValue::Float(f) => Value::Float64(*f),
                DefaultScalarValue::String(s) => Value::String(s.to_string()),
                DefaultScalarValue::Boolean(b) => Value::Bool(*b),
            })
        } else {
            None
        }
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

impl TryFrom<serde_json::Value> for Value {
    type Error = Error;

    fn try_from(value: serde_json::Value) -> Result<Value, Error> {
        match value {
            serde_json::Value::Array(a) => Ok(Value::Array(
                a.into_iter()
                    .map(|val| val.try_into())
                    .collect::<Result<Vec<_>, _>>()?,
            )),
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
                    Err(Error::TypeConversionFailed {
                        src: "serde_json::Value::Number".to_string(),
                        dst: "Value".to_string(),
                    })
                }
            }
            serde_json::Value::String(s) => Ok(Value::String(s)),
            serde_json::Value::Object(m) => Ok(Value::Map(
                m.into_iter()
                    .map(|(k, v)| {
                        let val = v.try_into()?;
                        Ok((k, val))
                    })
                    .collect::<Result<HashMap<String, Value>, Error>>()?,
            )),
        }
    }
}

impl TryFrom<Value> for bool {
    type Error = Error;

    fn try_from(value: Value) -> Result<bool, Self::Error> {
        if let Value::Bool(b) = value {
            Ok(b)
        } else {
            Err(Error::TypeConversionFailed {
                src: format!("{:#?}", value),
                dst: "bool".to_string(),
            })
        }
    }
}

impl TryFrom<Value> for f64 {
    type Error = Error;

    fn try_from(value: Value) -> Result<f64, Self::Error> {
        if let Value::Int64(i) = value {
            Ok(i as f64)
        } else if let Value::UInt64(i) = value {
            Ok(i as f64)
        } else if let Value::Float64(f) = value {
            Ok(f)
        } else {
            Err(Error::TypeConversionFailed {
                src: format!("{:#?}", value),
                dst: "f64".to_string(),
            })
        }
    }
}

impl TryFrom<Value> for i32 {
    type Error = Error;

    fn try_from(value: Value) -> Result<i32, Self::Error> {
        match value {
            Value::Int64(i) => Ok(i32::try_from(i)?),
            Value::UInt64(i) => Ok(i32::try_from(i)?),
            _ => Err(Error::TypeConversionFailed {
                src: format!("{:#?}", value),
                dst: "i32".to_string(),
            }),
        }
    }
}

impl TryFrom<Value> for String {
    type Error = Error;

    fn try_from(value: Value) -> Result<String, Self::Error> {
        if let Value::String(s) = value {
            Ok(s)
        } else {
            Err(Error::TypeConversionFailed {
                src: format!("{:#?}", value),
                dst: "String".to_string(),
            })
        }
    }
}

impl TryFrom<Value> for serde_json::Value {
    type Error = Error;

    fn try_from(value: Value) -> Result<serde_json::Value, Error> {
        match value {
            Value::Array(a) => Ok(serde_json::Value::Array(
                a.into_iter()
                    .map(|v| v.try_into())
                    .collect::<Result<Vec<_>, Error>>()?,
            )),
            Value::Bool(b) => Ok(serde_json::Value::Bool(b)),
            Value::Float64(f) => Ok(serde_json::Value::Number(
                serde_json::Number::from_f64(f).ok_or_else(|| Error::TypeConversionFailed {
                    src: "Value::Float64".to_string(),
                    dst: "serde_json::Number".to_string(),
                })?,
            )),
            Value::Int64(i) => Ok(serde_json::Value::Number(i.into())),
            Value::Map(hm) => Ok(serde_json::Value::Object(
                hm.into_iter()
                    .map(|(k, v)| {
                        let val = v.try_into()?;
                        Ok((k, val))
                    })
                    .collect::<Result<serde_json::Map<String, serde_json::Value>, Error>>()?,
            )),
            Value::Null => Ok(serde_json::Value::Null),
            Value::String(s) => Ok(serde_json::Value::String(s)),
            Value::UInt64(i) => Ok(serde_json::Value::Number(i.into())),
        }
    }
}

impl<T> TryFrom<Value> for Vec<T>
where
    T: TryFrom<Value, Error = Error>,
{
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Array(a) = value {
            if let Some(Value::Null) = a.get(0) {
                // If the array null values, return an empty vector, indicating null to Juniper.
                Ok(Vec::new())
            } else {
                // If the array is other than null, try to do the conversation to a Vector.
                a.into_iter()
                    .map(|v| v.try_into())
                    .collect::<Result<Vec<_>, Error>>()
            }
        } else {
            Err(Error::TypeConversionFailed {
                src: format!("{:#?}", value),
                dst: "<T> where T: TryFrom<Value, Error = Error>".to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Value;

    /// Passes if the Value implements the Send trait
    #[test]
    fn test_value_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Value>();
    }

    /// Passes if Value implements the Sync trait
    #[test]
    fn test_value_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Value>();
    }
}
