output "lambda_function_url" {
  value = module.api_lambda.lambda_function_url
}

output "cloudfront_url" {
  value = "https://${aws_cloudfront_distribution.this.domain_name}/"
}

output "miniaturs_deployed_url" {
  value = "https://${var.subdomain}.${var.domain}"
}

output "miniaturs_shared_secret" {
  value = nonsensitive(random_password.miniaturs_shared_secret.result)
}