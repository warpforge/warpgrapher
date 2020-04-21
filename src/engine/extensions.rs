use crate::engine::context::WarpgrapherRequestContext;
use std::collections::hash_map::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// Trait implemented by warpgrapher extensions. Exposes hook points that allow
/// external logic to be executed during various points in the warpgrapher cycle
pub trait Extension<GlobalCtx, ReqCtx>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    ReqCtx: 'static + Clone + Sync + Send + Debug + WarpgrapherRequestContext,
{
    fn pre_request_hook(
        &self,
        _global_ctx: Option<GlobalCtx>,
        _req_ctx: Option<&mut ReqCtx>,
        _headers: &HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
        Ok(())
    }

    fn post_request_hook(
        &self,
        _global_ctx: Option<GlobalCtx>,
        _req_ctx: Option<&ReqCtx>,
        _response: &mut serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
        Ok(())
    }

    // TODO: Feature: resolver hooks
    // TODO: Feature: config transform hooks
    // TODO: Feature: types/endpoints
}

/// Type alias for a thread-safe Extension container.
pub type WarpgrapherExtensions<GlobalCtx, ReqCtx> =
    Vec<Arc<dyn Extension<GlobalCtx, ReqCtx> + Send + Sync>>;
