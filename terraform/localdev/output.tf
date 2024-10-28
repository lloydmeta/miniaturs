output "miniaturs_shared_secret" {
  value = nonsensitive(random_password.miniaturs_shared_secret.result)
}

output "lambda_function_url" {
  value = module.api_lambda.lambda_function_url
}

output "unprocessed_image_bucket_name" {
  value = module.image_cache.unprocessed_images_bucket
}

output "processed_image_bucket_name" {
  value = module.image_cache.processed_images_bucket
}
