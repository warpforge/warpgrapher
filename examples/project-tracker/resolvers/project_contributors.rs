extern crate warpgrapher;
use serde_json::{json, Value};
use std::collections::HashMap;
use self::warpgrapher::engine::context::GraphQLContext;
use self::warpgrapher::engine::objects::{Node, Rel, Object};
use self::warpgrapher::engine::schema::Info;
use self::warpgrapher::juniper::{Arguments, ExecutionResult, Executor, FieldError};
use warpgrapher::engine::context::RequestContext;
use warpgrapher::engine::resolvers::{ResolverContext};
use std::fmt::Debug;

pub fn resolver(
    context: ResolverContext<crate::AppGlobalContext, crate::AppRequestContext>
) -> ExecutionResult {

    context.return_rel(
        "7747474747",
        None,
        Node::new(
            "User".to_string(), 
            json!({
                "id": "566",
                "name": "Joe"
            }).as_object().unwrap().clone()
        )
    ) 

}
