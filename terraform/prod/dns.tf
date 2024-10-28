data "cloudflare_zones" "miniaturs" {
  filter {
    name   = var.domain
    status = "active"
    paused = false
  }
}

resource "cloudflare_record" "miniaturs" {
  name       = var.subdomain
  zone_id    = data.cloudflare_zones.miniaturs.zones.0.id
  type       = "CNAME"
  content    = aws_cloudfront_distribution.this.domain_name
  proxied    = true
  depends_on = [aws_acm_certificate_validation.miniaturs]
}
