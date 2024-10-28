# A Role that allows lambdas to assume different roles
resource "aws_iam_role" "lambda_iam_role" {
  name = "${var.function_name}-iam-role"

  assume_role_policy = <<POLICY
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Action": "sts:AssumeRole",
      "Principal": {
        "Service": "lambda.amazonaws.com"
      },
      "Effect": "Allow",
      "Sid": ""
    }
  ]
}
POLICY
}

# Role policy that allows some critical things needed to 
# run lambdas (logs specifically)
resource "aws_iam_role_policy" "lambda_access_policy" {
  name   = "${var.function_name}-access-policy"
  role   = aws_iam_role.lambda_iam_role.id
  policy = <<POLICY
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "logs:CreateLogStream",
        "logs:CreateLogGroup",
        "logs:PutLogEvents"
      ],
      "Resource": "*"
    }
  ]
}
POLICY
}