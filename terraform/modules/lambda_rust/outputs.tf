
output "lambda_function_url" {
  value = aws_lambda_function_url.this.function_url
}

output "lambda_function_name" {
  value = aws_lambda_function.this.function_name
}

# Used for cache-busing in prod
output "lambda_archive_checksum" {
  value = aws_lambda_function.this.code_sha256
}

output "lambda_role_name" {
  value = aws_iam_role.lambda_iam_role.name
}

output "lambda_role_arn" {
  value = aws_iam_role.lambda_iam_role.arn
}
