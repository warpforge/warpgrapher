//! Contains types and functions for application specific extensions to the Warpgrapher framework.

use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::database::DatabasePool;

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
/// # use warpgrapher::engine::context::{GlobalContext, RequestContext};
/// # use warpgrapher::engine::extensions::{Extension, Extensions};
///
/// #[derive(Clone, Debug)]
/// pub struct MetadataExtension<GlobalCtx, RequestCtx>
/// where
///     GlobalCtx: GlobalContext,
///     RequestCtx: RequestContext
/// {
///     _gctx: PhantomData<GlobalCtx>,
///     _rctx: PhantomData<RequestCtx>,
/// }
///
/// impl<GlobalCtx, RequestCtx> Extension<GlobalCtx, RequestCtx>
///     for MetadataExtension<GlobalCtx, RequestCtx>
/// where
///     GlobalCtx: GlobalContext,
///     RequestCtx: RequestContext,
/// {
///
///    fn pre_request_hook(
///         &self,
///         _global_ctx: Option<&GlobalCtx>,
///         request_ctx: RequestCtx,
///         _headers: &HashMap<String, String>,
///     ) -> Result<RequestCtx, Box<dyn std::error::Error + Sync + Send>> {
///        // Set values in request context, or take some other action
///        Ok(request_ctx)
///     }
/// }
/// ```
pub trait Extension<GlobalCtx, RequestCtx>: Debug + Send + Sync
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    fn pre_request_hook(
        &self,
        _op_name: Option<String>,
        _global_ctx: Option<&GlobalCtx>,
        request_ctx: RequestCtx,
        _headers: &HashMap<String, String>,
        _db_pool: DatabasePool
    ) -> Result<RequestCtx, Box<dyn std::error::Error + Sync + Send>> {
        Ok(request_ctx)
    }

    fn post_request_hook(
        &self,
        _global_ctx: Option<&GlobalCtx>,
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
/// # use warpgrapher::engine::context::{GlobalContext, RequestContext};
/// # use warpgrapher::engine::extensions::{Extension, Extensions};
///
/// #[derive(Clone, Debug)]
/// pub struct MetadataExtension<GlobalCtx, RequestCtx>
/// where
///     GlobalCtx: GlobalContext,
///     RequestCtx: RequestContext
/// {
///     _gctx: PhantomData<GlobalCtx>,
///     _rctx: PhantomData<RequestCtx>,
/// }
///
/// impl<GlobalCtx, RequestCtx> Extension<GlobalCtx, RequestCtx>
///     for MetadataExtension<GlobalCtx, RequestCtx>
/// where
///     GlobalCtx: GlobalContext,
///     RequestCtx: RequestContext,
/// {
///
///    fn pre_request_hook(
///         &self,
///         _global_ctx: Option<&GlobalCtx>,
///         request_ctx: RequestCtx,
///         _headers: &HashMap<String, String>,
///     ) -> Result<RequestCtx, Box<dyn std::error::Error + Sync + Send>> {
///        // Set values in request context, or take some other action
///        Ok(request_ctx)
///     }
/// }
///
/// let metadata_extension = MetadataExtension { _gctx: PhantomData, _rctx: PhantomData };
/// let extensions: Extensions<(), ()> = vec![Arc::new(metadata_extension)];
/// ```
pub type Extensions<GlobalCtx, RequestCtx> = Vec<Arc<dyn Extension<GlobalCtx, RequestCtx>>>;
