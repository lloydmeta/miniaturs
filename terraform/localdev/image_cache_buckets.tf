module "image_cache" {
  source = "../modules/file_store"

  processed_images_bucket   = var.image_cache_processed_bucket_name
  unprocessed_images_bucket = var.image_cache_unprocessed_bucket_name
  lambda_iam_role_name      = module.api_lambda.lambda_role_name
}
