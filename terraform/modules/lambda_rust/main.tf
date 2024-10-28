# Build the lambda zip locally, based on whether or not there has been a 
# change in sha512 hash of the concatenated sha256 of each file in the source dir
resource "null_resource" "rust_build" {
  triggers = {
    code_diff = sha512(join("", [
      for file in fileset(var.rust_src_path, "**") : filesha256("${var.rust_src_path}/${file}")
      if !startswith(file, "target") && !endswith(file, ".DS_Store")
    ]))
  }

  provisioner "local-exec" {
    working_dir = var.rust_src_path
    command     = "cargo lambda build --release --arm64 --output-format zip"
  }
}

# Zip the above again so we can feed the base4sha to aws_lambda_function's source_code_hash to decide 
# whether or not an update is required.
# 
# Without this, there is a chance of a stale zip being uploaded
data "archive_file" "this" {
  type        = "zip"
  source_file = "${var.rust_src_path}/../target/lambda/${var.cargo_lambda_env_name}/bootstrap.zip"
  output_path = "${var.rust_src_path}/../target/lambda/${var.cargo_lambda_env_name}/bootstrap_archive.zip"
  depends_on = [
    null_resource.rust_build
  ]
}

resource "aws_lambda_function" "this" {
  function_name = var.function_name

  filename         = "${var.rust_src_path}/../target/lambda/${var.cargo_lambda_env_name}/bootstrap.zip"
  source_code_hash = data.archive_file.this.output_base64sha256

  role          = aws_iam_role.lambda_iam_role.arn
  architectures = ["arm64"]
  handler       = "bootstrap"
  runtime       = "provided.al2023" # otherwise /var/task/bootstrap: /lib64/libc.so.6: version `GLIBC_2.28' not found (required by /var/task/bootstrap) 

  timeout = 30

  environment {
    variables = var.environment_variables
  }


  logging_config {
    log_format = "Text"
    log_group  = aws_cloudwatch_log_group.this.name
  }
  depends_on = [aws_cloudwatch_log_group.this]
}

resource "aws_lambda_function_url" "this" {
  function_name      = aws_lambda_function.this.function_name
  authorization_type = "NONE" # Allows for unsigned payloads from clients...
  # Ensure not just anyone can call it (it will be called through cloudfront..)
  # Doesn't seem to work with URLS that have _another_ schema stanza in them (which we need 😭)
  # authorization_type = "AWS_IAM"
}
