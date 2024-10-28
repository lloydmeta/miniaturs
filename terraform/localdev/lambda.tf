module "api_lambda" {
  source = "./../modules/lambda_rust"

  function_name = "${var.name}-api"

  rust_src_path         = "../../server"
  cargo_lambda_env_name = "miniaturs_server"

  environment_variables = {
    RUST_BACKTRACE            = 1
    MINIATURS_SHARED_SECRET   = random_password.miniaturs_shared_secret.result
    UNPROCESSED_IMAGES_BUCKET = module.image_cache.unprocessed_images_bucket
    PROCESSED_IMAGES_BUCKET   = module.image_cache.processed_images_bucket
  }
}
