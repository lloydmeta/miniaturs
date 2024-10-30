# Miniaturs
[![Continuous integration](https://github.com/lloydmeta/miniaturs/actions/workflows/ci.yaml/badge.svg)](https://github.com/lloydmeta/miniaturs/actions/workflows/ci.yaml)

HTTP image resizer

## Goals

* Secure 
* Fast: 
  * Startup should be in the low 2-digit ms range (e.g., avoid "oh, it's a lambda")
  * Processing should be quick
* Cheap: 
  * Pay as little as possible and avoid unnecessary work
  * Being fast can help minimise costs
* Scalable: can handle lots of requests
* Thumbor-ish
* A good net citizen (don’t make requests to third parties if we have it in cache)
* Debuggable

To fulfil the above:

* Runs in a Lambda
* Rust ⚡️
* Caching in layers: CDN with S3 for images
* Serverless, but built on HTTP framework ([cargo-lambda](https://www.cargo-lambda.info) on top of [axum](https://github.com/tokio-rs/axum))

An example Terraform config in `terraform/prod` is provided to show how to deploy at a subdomain using Cloudflare as our (free!) CDN + WAF.

## Usage:

We only support resizing at the moment

1. An "image" endpoint [a la Thumbor](https://thumbor.readthedocs.io/en/latest/usage.html#image-endpoint)
2. A "metadata" endpoint [a la Thumbor](https://thumbor.readthedocs.io/en/latest/usage.html#metadata-endpoint)
    * Difference: target image size is _not_ returned (might change in the future)

## Flow

1. Layer 1 validations (is the request well-formed?)
2. Ensure trusted request (e.g., check hash)
3. Layer 2 validations (no I/O):
    1. Are the image processing options supported?
       1. Resize-to target size check (e.g., is it too big?) PENDING
    2. Is the remote image path pointing to a supported source? PENDING
    3. Is the remote image extension supported?
4. Determine if we can return a cached result:
    1. Is there a cached result in the storage bucket?
        1. If yes, return it as the result
        2. Else continue
5. Image retrieval:
    1. Is the remote image already cached in our source bucket?
        1. If yes, retrieve it
        2. If not, issue a HEAD request to get image size PENDING
        3. If the image size does not exceed the configured max, retrieve it
            1. Else return an error  PENDING
    2. Is the actual downloaded image too big? PENDING
        1. If yes, return an error
6. Image processing:
    1. Is the image in a supported format for our processor?
        1. If yes, process it
        2. Else return an error
7. Cache the resulting image in our bucket
8. Return the resulting image

## Development

### Rust

Assuming we have the [Rust toolbelt installed](https://doc.rust-lang.org/cargo/getting-started/installation.html#install-rust-and-cargo), the main thing we need is `cargo-lambda`

```sh
❯ brew tap cargo-lambda/cargo-lambda
```

### AWS

* `brew install awscli` to install the CLI
* Log into your app

Ensure:

* `aws configure sso` is done
* `.aws/config` has the correct profile configuration, with a `[profile ${PROFILE_NAME}]` line where `PROFILE_NAME` matches what is in `main.tf`

#### Login for Terraform

`aws sso login --profile ${PROFILE_NAME}`

### Cloudflare

Ensure `CLOUDFLARE_API_TOKEN` is defined in the environment (needed for Cloudflare provider and cache busting). It’ll need privileges for updating DNS and cache settings.

## Deploying

### Terraform

* Use tfenv: https://formulae.brew.sh/formula/tfenv
* Check what version is needed and install using ^

* For local dev, `localstack` is used (see terraform/localdev/docker-compose.yaml), and `tflocal` is used (https://formulae.brew.sh/formula/terraform-local)
  * `docker-compose` through official Docker _or_ Rancher is supported, but [enabling admin access](https://github.com/rancher-sandbox/rancher-desktop/issues/2534#issuecomment-1909912585) is needed for running tests with Rancher

### Per Environment

Use `Makefile` targets.

* For local dev:
    * `make start_dev_env provision_dev_env`
    * `make begin_dev`
    * `TO_SIGN="200x-100/https://beachape.com/images/octopress_with_container.png" make signature_for_localstack` to get a signed path for dev env
    * `TO_SIGN="200x-100/https://beachape.com/images/octopress_with_container.png" make signature_for_dev` to get a signed path for dev
* For prod:
    * Copy and customise:
      * `main.tf.example` to `main.tf`
      * `terraform.tfvars.example` to `terraform.tfvars`
    * `make plan_prod` to see changes
    * `make provision_prod` to apply changes
    * `TO_SIGN="200x-100/https://beachape.com/images/octopress_with_container.png" make signature_for_prod` to get a signed path

## To Explore

* Image resizing 
  * https://imgproxy.net/blog/almost-free-image-processing-with-imgproxy-and-aws-lambda/
  * https://zenn.dev/devneko/articles/0a6fb5c9ea5689
  * https://crates.io/crates/image
* [Logs, tracing](https://github.com/tokio-rs/tracing?tab=readme-ov-file#in-applications)
* Improve image resizing:
  * Encapsulate and test
  * Do in another thread?