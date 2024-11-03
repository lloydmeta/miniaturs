
variable "function_name" {
  description = "The name of the Lambda function."
  type        = string
}


variable "environment_variables" {
  description = "A map of environment variables to pass to the Lambda function."
  type        = map(string)
  default     = {}
}

variable "rust_src_path" {
  description = "The path to the Lambda function's Rust project."
  type        = string
}

variable "cargo_lambda_env_name" {
  description = "name in cargo lambda new [name]"
  type        = string
}

variable "log_retention_days" {
  description = "Days to keep lambda logs"
  default     = 14
  type        = number
}

variable "memory_size_mb" {
  description = "Memory to allocate for lambda ($$)"
  default     = 256
  type        = number
}
