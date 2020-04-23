use serde_json::json;
use warpgrapher::engine::objects::Node;
use warpgrapher::engine::resolvers::ResolverContext;
use warpgrapher::juniper::ExecutionResult;

pub fn resolver(
    context: ResolverContext<crate::AppGlobalContext, crate::AppRequestContext>,
) -> ExecutionResult {
    context.return_rel(
        "7747474747",
        None,
        Node::new(
            "User".to_string(),
            json!({
                "id": "566",
                "name": "Joe"
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
    )
}
