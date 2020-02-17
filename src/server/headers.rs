///! This is a temporary implementation until we can submit a PR to actix_web
use actix_http::error::Error;
use actix_web::dev::Payload;
use actix_web::error::QueryPayloadError;
use actix_web::{FromRequest, HttpRequest};
use futures::future::{ok, Ready};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// This struct allows HTTP requests headers to be accessed inside
/// an actix_web request handler.
#[derive(Debug)]
pub struct Headers {
    pub data: HashMap<String, String>,
}

impl Headers {
    fn new() -> Headers {
        Headers {
            data: HashMap::new(),
        }
    }
}

impl fmt::Display for Headers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "N/A")
    }
}

impl FromRequest for Headers {
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;
    type Config = HeadersConfig;

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let mut h = Headers::new();
        h.data = HashMap::new();
        for (key, value) in req.headers().iter() {
            if let Ok(s) = value.to_str() {
                h.data.insert(key.as_str().to_string(), s.to_string());
            }
        }
        ok(h)
    }
}

#[derive(Clone)]
pub struct HeadersConfig {
    ehandler: Option<Arc<dyn Fn(QueryPayloadError, &HttpRequest) -> Error + Send + Sync>>,
}

impl Default for HeadersConfig {
    fn default() -> Self {
        HeadersConfig { ehandler: None }
    }
}
