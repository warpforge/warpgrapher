use warpgrapher::engine::resolvers::ResolverContext;
use warpgrapher::juniper::ExecutionResult;

pub fn resolver(
    context: ResolverContext<crate::AppGlobalContext, crate::AppRequestContext>,
) -> ExecutionResult {
    // get projects from database
    let db = context.executor.context().pool.get().unwrap();
    let query = "MATCH (n:Project) RETURN (n);";
    let results = db.exec(query).unwrap(); // TODO: extract correctly

    // return number of projects
    let count = results.data.len();
    context.return_scalar(count as i32)
}
