use warpgrapher::engine::resolvers::ResolverContext;
use warpgrapher::juniper::ExecutionResult; // TODO: consider re-exporting from warpgrapher::engine::resolvers

pub fn resolver(
    context: ResolverContext<crate::AppGlobalContext, crate::AppRequestContext>,
) -> ExecutionResult {
    context.return_scalar(1)
}
