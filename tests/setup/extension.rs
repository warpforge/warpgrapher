use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use warpgrapher::engine::context::RequestContext;
use warpgrapher::engine::database::DatabasePool;
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
pub struct MetadataExtension<RequestCtx>
where
    RequestCtx: 'static + Clone + Sync + Send + Debug + RequestContext,
{
    _rctx: PhantomData<RequestCtx>,
}

impl<RequestCtx> MetadataExtension<RequestCtx>
where
    RequestCtx: 'static + Clone + Sync + Send + Debug + RequestContext + MetadataExtensionCtx,
{
    #[allow(dead_code)]
    pub fn new() -> MetadataExtension<RequestCtx> {
        MetadataExtension { _rctx: PhantomData }
    }
}

impl<RequestCtx> Extension<RequestCtx> for MetadataExtension<RequestCtx>
where
    RequestCtx: RequestContext + MetadataExtensionCtx,
{
    /// Request hook that executes prior to a request being handled by the GraphQL executor.
    /// This hook will add metadata into the request context.
    fn pre_request_hook(
        &self,
        _op_name: Option<String>,
        mut req_ctx: RequestCtx,
        _headers: &HashMap<String, String>,
        _db_pool: DatabasePool,
    ) -> Result<RequestCtx, Box<dyn std::error::Error + Sync + Send>> {
        req_ctx.set_metadata(Metadata {
            src_ip: "1.2.3.4".to_string(),
            src_useragent: "Firefox-123".to_string(),
        });

        Ok(req_ctx)
    }
}
