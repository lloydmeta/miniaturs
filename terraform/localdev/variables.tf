variable "name" {
  description = "Name to be used for deploying the lambda"
  default     = "miniaturs"
  type        = string
}

variable "image_cache_unprocessed_bucket_name" {
  description = "Name of the S3 bucket to use for unprocessed images"
  default     = "unprocessed-images"
  type        = string
}

variable "image_cache_processed_bucket_name" {
  description = "Name of the S3 bucket to use for processed images"
  default     = "processed-images"
  type        = string
}
