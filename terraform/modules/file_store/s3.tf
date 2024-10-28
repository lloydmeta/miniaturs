resource "aws_s3_bucket" "processed" {
  bucket = var.processed_images_bucket
}

resource "aws_s3_bucket" "unprocessed" {
  bucket = var.unprocessed_images_bucket
}


resource "aws_s3_bucket_lifecycle_configuration" "processed" {
  bucket = aws_s3_bucket.processed.id
  rule {
    status = "Enabled"
    id     = "expire_all_files"
    expiration {
      days = var.bucket_expiration_days
    }
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "unprocessed" {
  bucket = aws_s3_bucket.unprocessed.id
  rule {
    status = "Enabled"
    id     = "expire_all_files"
    expiration {
      days = var.bucket_expiration_days
    }
  }
}
