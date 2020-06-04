//! This module provides types for custom input validation

use crate::engine::value::Value;
use crate::Error;
use std::collections::HashMap;

pub type ValidatorFunc = fn(&Value) -> Result<(), Error>;

pub type Validators = HashMap<String, Box<ValidatorFunc>>;
