resource "cloudflare_ruleset" "miniaturs_assets" {
  zone_id = data.cloudflare_zones.miniaturs.zones.0.id
  kind    = "zone"
  name    = "default"
  phase   = "http_request_cache_settings"
  rules {
    action      = "set_cache_settings"
    description = "Cache everything"
    enabled     = true
    expression  = "(http.host eq \"${var.subdomain}.${var.domain}\")"

    action_parameters {
      # https://developers.cloudflare.com/cache/how-to/cache-rules/settings/#edge-ttl
      edge_ttl {
        mode = "respect_origin"

        status_code_ttl {
          status_code_range {
            to = 499
          }
          value = 31536000
          # value = 0
        }
        status_code_ttl {
          status_code_range {
            from = 500
          }
          value = 60
          # value = 0
        }

      }
      cache = true
    }
  }
}

resource "null_resource" "miniaturs_cache_bust" {
  triggers = {
    code_diff = module.api_lambda.lambda_archive_checksum
  }

  provisioner "local-exec" {
    command = "curl -XPOST https://api.cloudflare.com/client/v4/zones/${data.cloudflare_zones.miniaturs.zones.0.id}/purge_cache  --header 'Content-Type: application/json' --header \"Authorization: Bearer $CLOUDFLARE_API_TOKEN\"  --data '{ \"purge_everything\": true }'"
  }

}
