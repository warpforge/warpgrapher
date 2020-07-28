//! This module provides types for custom input validation

use crate::engine::value::Value;
use crate::Error;
use std::collections::HashMap;

/// Type alias for a custom function used to validate the input to a resolver
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::validators::{ValidatorFunc, Validators};
/// # use warpgrapher::engine::value::Value;
///
/// fn name_validator(value: &Value) -> Result<(), Error> {
///     let name = match value {
///         Value::Map(m) => match m.get("name") {
///             Some(n) => n,
///             None => return Err(Error::ValidationFailed {message: "Name missing.".to_string()}),
///         },
///         _ => return Err(Error::ValidationFailed {message: "Field map missing.".to_string()}),
///     };
///
///     match name {
///         Value::String(s) => if s == "KENOBI" {
///                 return Err(Error::ValidationFailed {
///                     message: "Cannot be named KENOBI.".to_string()
///                 });
///             } else {
///                 return Ok(())
///             },
///         _ => Err(Error::ValidationFailed {message: "Expected a string value.".to_string()}),
///     }
/// }
///
/// let f: Box<ValidatorFunc> = Box::new(name_validator);
/// ```
pub type ValidatorFunc = fn(&Value) -> Result<(), Error>;

/// Type alias for a custom function used to validate the input to a resolver
///
/// Examples
///
/// ```rust
/// # use warpgrapher::engine::validators::{ValidatorFunc, Validators};
/// # use warpgrapher::engine::value::Value;
/// # use warpgrapher::Error;
///
/// fn name_validator(value: &Value) -> Result<(), Error> {
///     let name = match value {
///         Value::Map(m) => match m.get("name") {
///             Some(n) => n,
///             None => return Err(Error::ValidationFailed {message: "Name missing.".to_string()}),
///         },
///         _ => return Err(Error::ValidationFailed {message: "Field map missing.".to_string()}),
///     };
///
///     match name {
///         Value::String(s) => if s == "KENOBI" {
///                 return Err(Error::ValidationFailed {
///                     message: "Cannot be named KENOBI.".to_string()
///                 });
///             } else {
///                 return Ok(());
///             },
///         _ => Err(Error::ValidationFailed {message: "Expected a string value.".to_string()}),
///      }
/// }
///
/// let mut validators = Validators::new();
/// validators.insert("name_validator".to_string(), Box::new(name_validator));
/// ```
pub type Validators = HashMap<String, Box<ValidatorFunc>>;
