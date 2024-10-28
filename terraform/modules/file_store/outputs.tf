output "processed_images_bucket" {
  value = aws_s3_bucket.processed.bucket
}

output "unprocessed_images_bucket" {
  value = aws_s3_bucket.unprocessed.bucket
}