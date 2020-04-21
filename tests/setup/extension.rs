use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use warpgrapher::engine::context::WarpgrapherRequestContext;
use warpgrapher::engine::extensions::Extension;

/// Additional information about a request
#[derive(Clone, Debug)]
pub struct Metadata {
    pub src_ip: String,
    pub src_useragent: String,
}

/// Trait that must be implemented by app's request context struct
pub trait MetadataExtensionCtx {
    fn set_metadata(&mut self, metadata: Metadata) -> ();
}

/// Extension that adds metadata to request
#[derive(Clone)]
pub struct MetadataExtension<GlobalCtx, ReqCtx>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    ReqCtx: 'static + Clone + Sync + Send + Debug + WarpgrapherRequestContext,
{
    _gctx: PhantomData<GlobalCtx>,
    _rctx: PhantomData<ReqCtx>,
}

impl<GlobalCtx, ReqCtx> MetadataExtension<GlobalCtx, ReqCtx>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    ReqCtx:
        'static + Clone + Sync + Send + Debug + WarpgrapherRequestContext + MetadataExtensionCtx,
{
    pub fn new() -> MetadataExtension<GlobalCtx, ReqCtx> {
        MetadataExtension {
            _gctx: PhantomData,
            _rctx: PhantomData,
        }
    }
}

impl<GlobalCtx, ReqCtx> Extension<GlobalCtx, ReqCtx> for MetadataExtension<GlobalCtx, ReqCtx>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    ReqCtx:
        'static + Clone + Sync + Send + Debug + WarpgrapherRequestContext + MetadataExtensionCtx,
{
    /// Request hook that executes prior to a request being handled by the GraphQL executor.
    /// This hook will add metadata into the request context.
    fn pre_request_hook(
        &self,
        _global_ctx: Option<GlobalCtx>,
        req_ctx: Option<&mut ReqCtx>,
        _headers: &HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
        if let Some(rc) = req_ctx {
            rc.set_metadata(Metadata {
                src_ip: "1.2.3.4".to_string(),
                src_useragent: "Firefox-123".to_string(),
            });
        }
        Ok(())
    }
}
