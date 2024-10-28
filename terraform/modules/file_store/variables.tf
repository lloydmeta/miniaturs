variable "unprocessed_images_bucket" {
  description = "Bucket where unprocessed images are stored"
  type        = string
}

variable "processed_images_bucket" {
  description = "Bucket where processed images are stored"
  type        = string
}

variable "lambda_iam_role_name" {
  description = "Role name of the lambda IAM role to attach policies to"
  type        = string
}

variable "bucket_expiration_days" {
  description = "Number of days to keep images in buckets"
  default     = 1
  type        = number
}
