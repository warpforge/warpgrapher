//! Contains types and functions for application specific extensions to the Warpgrapher framework.

use crate::engine::config::Configuration;
use crate::engine::context::RequestContext;
use crate::engine::database::DatabaseEndpoint;

use std::collections::hash_map::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// Trait implemented by warpgrapher extensions. Exposes hook points that allow external logic to
/// be executed during various points in the warpgrapher request lifecycle
///
/// # Examples
///
/// ```rust
///
/// # use std::collections::HashMap;
/// # use std::marker::PhantomData;
/// # use warpgrapher::engine::context::RequestContext;
/// # use warpgrapher::engine::database::DatabaseEndpoint;
/// # use warpgrapher::engine::extensions::{Extension, Extensions};
///
/// #[derive(Clone, Debug)]
/// pub struct MetadataExtension<RequestCtx>
/// where
///     RequestCtx: RequestContext
/// {
///     _rctx: PhantomData<RequestCtx>,
/// }
///
/// impl<RequestCtx> Extension<RequestCtx> for MetadataExtension<RequestCtx>
/// where
///     RequestCtx: RequestContext,
/// {
///
///    fn pre_request_hook(
///         &self,
///         _op_name: Option<String>,
///         request_ctx: RequestCtx,
///         _headers: &HashMap<String, String>,
///         _db_pool: <<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType,
///     ) -> Result<RequestCtx, Box<dyn std::error::Error + Sync + Send>> {
///        // Set values in request context, or take some other action
///        Ok(request_ctx)
///     }
/// }
/// ```
pub trait Extension<RequestCtx>: Debug + Send + Sync
where
    RequestCtx: RequestContext,
{
    fn pre_build_hook(
        &self,
        _config: &mut Configuration
    ) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
        Ok(())
    }

    fn pre_request_hook(
        &self,
        _op_name: Option<String>,
        request_ctx: RequestCtx,
        _headers: &HashMap<String, String>,
        _db_pool: <<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType,
    ) -> Result<RequestCtx, Box<dyn std::error::Error + Sync + Send>> {
        Ok(request_ctx)
    }

    fn post_request_hook(
        &self,
        _request_ctx: &RequestCtx,
        response: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Sync + Send>> {
        Ok(response)
    }
}

/// Type alias for a thread-safe Extension vector.
///
/// # Examples
///
/// ```rust
/// # use std::collections::HashMap;
/// # use std::marker::PhantomData;
/// # use std::sync::Arc;
/// # use warpgrapher::engine::context::RequestContext;
/// # use warpgrapher::engine::database::DatabaseEndpoint;
/// # use warpgrapher::engine::extensions::{Extension, Extensions};
///
/// #[derive(Clone, Debug)]
/// pub struct MetadataExtension<RequestCtx>
/// where
///     RequestCtx: RequestContext
/// {
///     _rctx: PhantomData<RequestCtx>,
/// }
///
/// impl<RequestCtx> Extension<RequestCtx>
///     for MetadataExtension<RequestCtx>
/// where
///     RequestCtx: RequestContext,
/// {
///
///    fn pre_request_hook(
///         &self,
///         _op_name: Option<String>,
///         request_ctx: RequestCtx,
///         _headers: &HashMap<String, String>,
///         _db_pool: <<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType,
///     ) -> Result<RequestCtx, Box<dyn std::error::Error + Sync + Send>> {
///        // Set values in request context, or take some other action
///        Ok(request_ctx)
///     }
/// }
///
/// let metadata_extension = MetadataExtension { _rctx: PhantomData };
/// let extensions: Extensions<()> = vec![Arc::new(metadata_extension)];
/// ```
pub type Extensions<RequestCtx> = Vec<Arc<dyn Extension<RequestCtx>>>;
