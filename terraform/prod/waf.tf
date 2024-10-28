resource "cloudflare_ruleset" "rate_limit" {
  zone_id = data.cloudflare_zones.miniaturs.zones.0.id
  name    = "Rate limits"
  kind    = "zone"
  phase   = "http_ratelimit"

  rules {
    action = "block"
    action_parameters {
      response {
        status_code  = 429
        content      = "{\"message\": \"You've been rate-limited, come back later..\"}"
        content_type = "application/json"
      }
    }
    ratelimit {
      characteristics     = ["ip.src", "cf.colo.id"]
      period              = 10
      requests_per_period = 100
      mitigation_timeout  = 10
    }
    expression  = "true"
    description = "Rate limit requests to ${var.subdomain}.${var.domain} when exceeding the threshold of 4xx responses"
    enabled     = true
  }
}
