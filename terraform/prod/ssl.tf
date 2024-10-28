## Origin cert using ACM

resource "aws_acm_certificate" "miniaturs" {
  provider                  = aws.virginia # required for CloudFront 
  domain_name               = var.domain
  subject_alternative_names = ["www.${var.domain}", "${var.subdomain}.${var.domain}"]
  validation_method         = "DNS"
  lifecycle {
    create_before_destroy = true
  }
}

## Validation DNS record on Cloudflare
resource "cloudflare_record" "miniaturs_validation" {
  for_each = {
    for dvo in aws_acm_certificate.miniaturs.domain_validation_options : dvo.domain_name => {
      name   = dvo.resource_record_name
      record = dvo.resource_record_value
      type   = dvo.resource_record_type
    }
  }

  name    = each.value.name
  comment = "For Origin cert validation purposes only"
  content = each.value.record
  ttl     = 60
  type    = each.value.type
  zone_id = data.cloudflare_zones.miniaturs.zones.0.id
}

## Validation on ACM
resource "aws_acm_certificate_validation" "miniaturs" {
  provider                = aws.virginia # required for CloudFront
  certificate_arn         = aws_acm_certificate.miniaturs.arn
  validation_record_fqdns = [for record in cloudflare_record.miniaturs_validation : record.hostname]
}

## Turn on full SSL on Cloudflare, otherwise you can'T reach it through CloudFlare
## https://stackoverflow.com/questions/75931089/cloudflare-terraform-enable-ssl-full-encryption
resource "cloudflare_zone_settings_override" "miniaturs" {
  zone_id = data.cloudflare_zones.miniaturs.zones.0.id
  settings {
    ssl = "full"
  }
}
