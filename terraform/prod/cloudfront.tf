resource "aws_cloudfront_distribution" "this" {
  aliases = ["${var.subdomain}.${var.domain}"]
  origin {
    domain_name              = regex("//(.*)/", module.api_lambda.lambda_function_url).0
    origin_access_control_id = aws_cloudfront_origin_access_control.this.id
    origin_id                = "${var.name}_lambda"
    custom_origin_config {
      http_port              = 80
      https_port             = 443
      origin_protocol_policy = "https-only"
      origin_ssl_protocols   = ["TLSv1", "TLSv1.1", "TLSv1.2"]
    }
  }

  enabled = true
  default_cache_behavior {
    allowed_methods  = ["HEAD", "DELETE", "POST", "GET", "OPTIONS", "PUT", "PATCH"]
    cached_methods   = ["HEAD", "GET", "OPTIONS"]
    target_origin_id = "${var.name}_lambda"
    forwarded_values {
      query_string = true
      cookies {
        forward = "none"
      }
    }

    viewer_protocol_policy = "redirect-to-https"
    min_ttl                = 0
    default_ttl            = 0
    max_ttl                = 0
    compress               = true
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
      locations        = []
    }
  }
  viewer_certificate {
    acm_certificate_arn = aws_acm_certificate.miniaturs.arn
    ssl_support_method  = "sni-only"
  }
}

# Amazon Original Access Control
# https://aws.amazon.com/blogs/networking-and-content-delivery/amazon-cloudfront-introduces-origin-access-control-oac/
# Not really used because CloudFront does not sign request bodies, so
# the aws_lambda_function_url.authorization_type is set to `None` to
# allow clients to send bodies
resource "aws_cloudfront_origin_access_control" "this" {
  name                              = "${var.name}-oac"
  signing_protocol                  = "sigv4"
  signing_behavior                  = "always"
  origin_access_control_origin_type = "lambda"
}


resource "aws_lambda_permission" "allow_cloudfront" {
  statement_id  = "AllowCloudFrontServicePrincipal"
  action        = "lambda:InvokeFunctionUrl"
  function_name = module.api_lambda.lambda_function_name
  principal     = "cloudfront.amazonaws.com"
  source_arn    = aws_cloudfront_distribution.this.arn
}
