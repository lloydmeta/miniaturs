variable "name" {
  description = "Name to be used for deploying the lambda"
  default     = "miniaturs"
  type        = string
}

variable "image_cache_unprocessed_bucket_name" {
  description = "Name of the S3 bucket to use for unprocessed images"
  default     = "miniaturs-unprocessed-images"
  type        = string
}

variable "image_cache_processed_bucket_name" {
  description = "Name of the S3 bucket to use for processed images"
  default     = "miniaturs-processed-images"
  type        = string
}

variable "subdomain" {
  description = "Subdomain to deploy the lambda to"
  type        = string
}

variable "domain" {
  description = "Domain to deploy the lambda to"
  type        = string
}

variable "memory_size_mb" {
  description = "Memory to allocate for lambda ($$)"
  default     = 512
  type        = number
}
