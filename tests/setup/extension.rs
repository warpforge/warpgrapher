use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use warpgrapher::engine::context::{GlobalContext, RequestContext};
use warpgrapher::engine::extensions::Extension;

/// Additional information about a request
#[derive(Clone, Debug)]
pub struct Metadata {
    pub(crate) src_ip: String,
    pub(crate) src_useragent: String,
}

/// Trait that must be implemented by app's request context struct
pub trait MetadataExtensionCtx {
    fn set_metadata(&mut self, metadata: Metadata);
}

/// Extension that adds metadata to request
#[derive(Clone, Debug)]
pub struct MetadataExtension<GlobalCtx, RequestCtx>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    RequestCtx: 'static + Clone + Sync + Send + Debug + RequestContext,
{
    _gctx: PhantomData<GlobalCtx>,
    _rctx: PhantomData<RequestCtx>,
}

impl<GlobalCtx, RequestCtx> MetadataExtension<GlobalCtx, RequestCtx>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    RequestCtx: 'static + Clone + Sync + Send + Debug + RequestContext + MetadataExtensionCtx,
{
    #[allow(dead_code)]
    pub fn new() -> MetadataExtension<GlobalCtx, RequestCtx> {
        MetadataExtension {
            _gctx: PhantomData,
            _rctx: PhantomData,
        }
    }
}

impl<GlobalCtx, RequestCtx> Extension<GlobalCtx, RequestCtx>
    for MetadataExtension<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext + MetadataExtensionCtx,
{
    /// Request hook that executes prior to a request being handled by the GraphQL executor.
    /// This hook will add metadata into the request context.
    fn pre_request_hook(
        &self,
        _global_ctx: Option<&GlobalCtx>,
        mut req_ctx: RequestCtx,
        _headers: &HashMap<String, String>,
    ) -> Result<RequestCtx, Box<dyn std::error::Error + Sync + Send>> {
        req_ctx.set_metadata(Metadata {
            src_ip: "1.2.3.4".to_string(),
            src_useragent: "Firefox-123".to_string(),
        });

        Ok(req_ctx)
    }
}
