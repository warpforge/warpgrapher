use serde_json::json;
use warpgrapher::engine::resolvers::{ExecutionResult, GraphNode, GraphRel, ResolverContext};

pub fn resolver(
    context: ResolverContext<crate::AppGlobalContext, crate::AppRequestContext>,
) -> ExecutionResult {
    context.resolve_rel(GraphRel {
        id: "1234567890",
        props: None,
        dst: GraphNode {
            typename: "User",
            props: json!({
                "id": "566",
                "name": "Joe"
            })
            .as_object()
            .unwrap(),
        },
    })
}
