use lambda_http::{run, tracing, Error};
use miniaturs_server::{
    api::routing::handlers::create_router,
    infra::{components::AppComponents, config::Config},
};

use std::env::set_var;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // If you use API Gateway stages, the Rust Runtime will include the stage name
    // as part of the path that your application receives.
    // Setting the following environment variable, you can remove the stage from the path.
    // This variable only applies to API Gateway stages,
    // you can remove it if you don't use them.
    // i.e with: `GET /test-stage/todo/id/123` without: `GET /todo/id/123`
    set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");

    // required to enable CloudWatch error logging by the runtime
    tracing::init_default_subscriber();

    let config = Config::load_env().await?;
    let app_components = AppComponents::create(config)?;

    let router = create_router(app_components);

    run(router).await
}
