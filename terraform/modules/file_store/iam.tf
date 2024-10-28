data "aws_iam_policy_document" "lambda_s3" {
  statement {
    actions = [
      "s3:*"
    ]

    resources = [
      "arn:aws:s3:::${var.processed_images_bucket}",
      "arn:aws:s3:::${var.processed_images_bucket}/*",
      "arn:aws:s3:::${var.unprocessed_images_bucket}",
      "arn:aws:s3:::${var.unprocessed_images_bucket}/*"
    ]
  }
}

resource "aws_iam_policy" "lambda_s3" {
  name        = "lambda-s3-permissions"
  description = "Contains S3 permissions for lambda"
  policy      = data.aws_iam_policy_document.lambda_s3.json
}

resource "aws_iam_role_policy_attachment" "lambda_s3" {
  role       = var.lambda_iam_role_name
  policy_arn = aws_iam_policy.lambda_s3.arn
}
