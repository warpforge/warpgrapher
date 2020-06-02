use crate::engine::context::{GlobalContext, RequestContext};
use std::collections::hash_map::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// Trait implemented by warpgrapher extensions. Exposes hook points that allow
/// external logic to be executed during various points in the warpgrapher cycle
pub trait Extension<GlobalCtx, ReqCtx>: Debug
where
    GlobalCtx: GlobalContext,
    ReqCtx: RequestContext,
{
    fn pre_request_hook(
        &self,
        _global_ctx: Option<&GlobalCtx>,
        _req_ctx: &mut ReqCtx,
        _headers: &HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
        Ok(())
    }

    fn post_request_hook(
        &self,
        _global_ctx: Option<&GlobalCtx>,
        _req_ctx: &ReqCtx,
        _response: &mut serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
        Ok(())
    }

    // TODO: Feature: resolver hooks
    // TODO: Feature: config transform hooks
    // TODO: Feature: types/endpoints
}

/// Type alias for a thread-safe Extension container.
pub type Extensions<GlobalCtx, ReqCtx> = Vec<Arc<dyn Extension<GlobalCtx, ReqCtx> + Send + Sync>>;
