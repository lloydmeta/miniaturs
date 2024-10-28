# Miniaturs 
[![Continuous integration](https://github.com/lloydmeta/miniaturs/actions/workflows/ci.yaml/badge.svg)](https://github.com/lloydmeta/miniaturs/actions/workflows/ci.yaml)

HTTP image resizer

## Goals

* Secure 
* Fast: 
  * Startup should be low 2 digit ms (e.g. avoid "oh, it's a lambda")
  * Processing should be quick
* Cheap: 
  * Pay as little as possible, and do as little as possible
  * Being fast can help avoid paying
* Scalable: can handle lots of requests
* Thumbor-ish 
* A good net-citizen (don't make requests to 3rd parties if we have it in cache)
* Debuggable

To fulfil the above:

* Runs in a lambda
* Rust ⚡️
* Caching in layers: CDN with S3 for images
* Serverless, but built on HTTP-framework 

A example TF in `terraform/prod` is provided to show how to deploy something that sits
at a subdomain, using Cloudflare as our (free!) CDN + WAF.

## Flow

1. Layer 1 validations (is the request well formed)
2. Ensure trusted request (e.g. check hash)
3. Layer 2 validations (no I/O)
  1. Are the image processing options supported?
    1. Resize-to target size check (e.g. is it too big?) PENDING
  2. Is the remote image path pointing to a supported source? PENDING
  3. Is the remote image extension supported?
4. Determine if we can return a cached result
  1. Is there a cached result in the storage bucket?
    1. If yes, return it as the result
    2. Else continue
5. Image retrieval:
  1. Is the remote image already cached in our source bucket?
    1. If yes, retrieve it
    2. If not, issue a HEAD request to get image size PENDING
      1. If the image size does not exceed configured max, retrieve it 
      2. Else return an error
    3. Is the actual downloadeded image too big? PENDING
      1. If yes, return an error
6. Image processing:
  1. Is the image in a supported format for our processor?
    1. If yes, process
    2. Else return an error
7. Cache resulting image in our bucket
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

Ensure 

* `aws configure sso` is done
* `.aws/config` has the right profile config, with a `[profile ${PROFILE_NAME}]` line, where `PROFILE_NAME` matches what is in `main.tf`


#### Login for Terraform

`aws sso login --profile ${PROFILE_NAME}`

### Cloudflare

* Ensure `CLOUDFLARE_API_TOKEN` is defined in the env (needed for Cloudflare provider and cache busting). It'll need the privileges for updating DNS and cache settings

## Deploying

### Terraform

* Use tfenv: https://formulae.brew.sh/formula/tfenv
* Check what version is needed an install using ^

* For local dev, `localstack` is used (see terraform/localdev/docker-compose.yaml), and `tflocal` is used (https://formulae.brew.sh/formula/terraform-local)
  * `docker-compose` through official docker _or_ Rancher is supported, but [enabling admin access](https://github.com/rancher-sandbox/rancher-desktop/issues/2534#issuecomment-1909912585) is needed for running tests with Rancher

### Per env

Use `Makefile`  targets

* For local def:
    * `make start_dev_env provision_dev_env`
    * `make begin_dev`
    * `TO_SIGN="200x-100/https://beachape.com/images/octopress_with_container.png" make signature_for_localstack` to get a signed path for devenv
    * `TO_SIGN="200x-100/https://beachape.com/images/octopress_with_container.png" make signature_for_dev` to get a signed path for dev
* For prod:
    * Copy + customise:
      * `main.tf.example` to `main.tf` 
      * `terraform.tfvars.example` to `terraform.tfvars`
    * `make plan_prod` to see changes
    * `make provision_prod` to apply changes
    * `TO_SIGN="200x-100/https://beachape.com/images/octopress_with_container.png" make signature_for_prod` to get a signed path

## To explore

* img resizing 
  * https://imgproxy.net/blog/almost-free-image-processing-with-imgproxy-and-aws-lambda/
  * https://zenn.dev/devneko/articles/0a6fb5c9ea5689
  * https://crates.io/crates/image
* a [metadata endpoint](https://thumbor.readthedocs.io/en/stable/usage.html#metadata-endpoint)
* [logs, tracing](https://github.com/tokio-rs/tracing?tab=readme-ov-file#in-applications)
* improve image resizing
  * Encapsulate + test
  * Do in another thread?