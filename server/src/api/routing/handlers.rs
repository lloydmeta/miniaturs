//! This is an example function that leverages the Lambda Rust runtime HTTP support
//! and the [axum](https://docs.rs/axum/latest/axum/index.html) web framework.  The
//! runtime HTTP support is backed by the [tower::Service](https://docs.rs/tower-service/0.3.2/tower_service/trait.Service.html)
//! trait.  Axum's applications are also backed by the `tower::Service` trait.  That means
//! that it is fairly easy to build an Axum application and pass the resulting `Service`
//! implementation to the Lambda runtime to run as a Lambda function.  By using Axum instead
//! of a basic `tower::Service` you get web framework niceties like routing, request component
//! extraction, validation, etc.
use std::any::Any;
use std::io::Cursor;

use axum::extract::{Path, State};
use axum::http::header::ACCEPT;
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use axum::{response::Json, routing::*, Router};

use image::{ImageFormat, ImageReader};
use reqwest::header::{CACHE_CONTROL, CONTENT_TYPE};
use responses::Standard;
use tower_http::catch_panic::CatchPanicLayer;

use crate::api::requests::{ImageResizePathParam, Signature};
use crate::api::responses;
use crate::infra::components::AppComponents;
use crate::infra::config::AuthenticationSettings;
use crate::infra::errors::AppError;
use crate::infra::image_caching::{
    ImageCacher, ImageFetchRequest, ImageFetchedCacheRequest, ImageResizeRequest,
    ImageResizedCacheRequest,
};

use miniaturs_shared::signature::{ensure_signature_is_valid_for_path_and_query, SignatureError};

const CACHE_CONTROL_HEADER_VALUE: HeaderValue = HeaderValue::from_static("max-age=31536000");

pub fn create_router(app_components: AppComponents) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/:signature/:resized_image/*image_url", get(resize))
        .fallback(handle_404)
        .layer(CatchPanicLayer::custom(handle_panic))
        .with_state(app_components)
}

async fn root() -> Json<Standard> {
    Json(Standard {
        message: "You probably want to use the resize url...".to_string(),
    })
}

async fn resize(
    State(app_components): State<AppComponents>,
    uri: Uri,
    Path((signature, resized_image, image_url)): Path<(Signature, ImageResizePathParam, String)>,
) -> Result<Response, AppError> {
    ensure_signature_is_valid(
        &app_components.config.authentication_settings,
        &uri,
        signature,
    )?;
    let processed_image_request = {
        ImageResizeRequest {
            requested_image_url: image_url.clone(),
            resize_target: resized_image.into(),
        }
    };
    let maybe_cached_resized_image = app_components
        .processed_images_cacher
        .get(&processed_image_request)
        .await;

    if let Some(cached_resized_image) = maybe_cached_resized_image {
        let mut response_headers = HeaderMap::new();
        // TODO see if we can read the type from the cached
        if let Ok(content_type_header) =
            HeaderValue::from_str(&cached_resized_image.requested.content_type)
        {
            response_headers.insert(CONTENT_TYPE, content_type_header);
        }
        Ok((StatusCode::OK, response_headers, cached_resized_image.bytes).into_response())
    } else {
        let unprocessed_cache_retrieve_req = ImageFetchRequest {
            requested_image_url: image_url.clone(),
        };

        let maybe_cached_fetched_image = app_components
            .unprocessed_images_cacher
            .get(&unprocessed_cache_retrieve_req)
            .await;

        let (response_status_code, bytes, maybe_content_type_string) =
            if let Some(cached_fetched) = maybe_cached_fetched_image {
                (
                    StatusCode::OK,
                    cached_fetched.bytes,
                    cached_fetched.requested.content_type,
                )
            } else {
                let mut proxy_response = app_components.http_client.get(&image_url).send().await?;
                let status_code = proxy_response.status();
                let headers = proxy_response.headers_mut();
                let maybe_content_type = headers.remove(CONTENT_TYPE);

                let maybe_content_type_string =
                    maybe_content_type.and_then(|h| h.to_str().map(|s| s.to_string()).ok());

                let cache_fetched_req = ImageFetchedCacheRequest {
                    request: unprocessed_cache_retrieve_req,
                    content_type: maybe_content_type_string.clone(),
                };
                let bytes: Vec<_> = proxy_response.bytes().await?.into();
                app_components
                    .unprocessed_images_cacher
                    .set(&bytes, &cache_fetched_req)
                    .await;

                let response_status_code = StatusCode::from_u16(status_code.as_u16())?;
                (response_status_code, bytes, maybe_content_type_string)
            };

        let mut image_reader = ImageReader::new(Cursor::new(bytes));

        let maybe_image_format_from_input = maybe_content_type_string
            .as_ref()
            .and_then(ImageFormat::from_mime_type)
            .or_else(|| ImageFormat::from_path(image_url).ok());

        let reader_with_format = if let Some(image_format) = maybe_image_format_from_input {
            image_reader.set_format(image_format);
            image_reader
        } else {
            image_reader.with_guessed_format()?
        };

        let format = reader_with_format
            .format()
            .ok_or(AppError::UnableToDetermineFormat)?;
        let mut dynamic_image = reader_with_format.decode()?;

        dynamic_image = dynamic_image.resize(
            resized_image.target_width as u32,
            resized_image.target_height as u32,
            image::imageops::FilterType::Gaussian,
        );

        if resized_image.target_width < 0 {
            dynamic_image = dynamic_image.fliph();
        }
        if resized_image.target_height < 0 {
            dynamic_image = dynamic_image.flipv();
        }

        let mut cursor = Cursor::new(Vec::new());
        dynamic_image.write_to(&mut cursor, format)?;
        let written_bytes = cursor.into_inner();

        let cache_image_req = ImageResizedCacheRequest {
            request: processed_image_request,
            content_type: format.to_mime_type().to_string(),
        };

        //cache the thing
        app_components
            .processed_images_cacher
            .set(&written_bytes, &cache_image_req)
            .await;

        let mut response_headers = HeaderMap::new();
        let maybe_content_type_header = maybe_content_type_string
            .as_ref()
            .and_then(|s| HeaderValue::from_str(s).ok());

        if let Some(content_type_header) = maybe_content_type_header {
            response_headers.insert(CONTENT_TYPE, content_type_header);
        }
        response_headers.insert(CACHE_CONTROL, CACHE_CONTROL_HEADER_VALUE);

        Ok((response_status_code, response_headers, written_bytes).into_response())
    }
}

/// Example on how to return status codes and data from an Axum function
async fn health_check() -> (StatusCode, Json<Standard>) {
    let health = true;
    match health {
        true => (
            StatusCode::OK,
            Json(Standard {
                message: "Healthy!".to_string(),
            }),
        ),
        false => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Standard {
                message: "Not Healthy!".to_string(),
            }),
        ),
    }
}

async fn handle_404(headers: HeaderMap) -> impl IntoResponse {
    match headers.get(ACCEPT).map(|x| x.to_str().unwrap_or("unknown")) {
        Some(s) if s.contains("text/html") => (
            StatusCode::NOT_FOUND,
            Html("<body><h1>Not found.</h1></body>").into_response(),
        ),
        _ => (
            StatusCode::NOT_FOUND,
            Json(Standard {
                message: "Not found.".to_string(),
            })
            .into_response(),
        ),
    }
}

fn ensure_signature_is_valid(
    auth_settings: &AuthenticationSettings,
    uri: &Uri,
    Signature(signature): Signature,
) -> Result<(), AppError> {
    let path_and_query = if uri.query().is_none() {
        uri.path()
    } else {
        uri.path_and_query()
            .map(|pq| {
                // lambda axum seems to insert empty query params when handling reqs
                // as a lambda
                let pq_as_str = pq.as_str();
                if uri.query().filter(|q| !q.trim().is_empty()).is_some() {
                    pq_as_str
                } else {
                    pq_as_str.strip_suffix("?").unwrap_or(pq_as_str)
                }
            })
            .unwrap_or("")
    };

    ensure_signature_is_valid_for_path_and_query(
        &auth_settings.shared_secret,
        path_and_query,
        &signature,
    )
    .map_err(|signature_err| match signature_err {
        SignatureError::CouldNotUseKey => AppError::CatchAll(anyhow::anyhow!(
            "Could not use the configured key. Maybe it's too long or too short"
        )),
        SignatureError::BadSignature => AppError::BadSignature(signature),
    })
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let result = match self {
            Self::CatchAll(anyhow_err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Standard {
                    message: anyhow_err.to_string(),
                }),
            ),
            Self::BadSignature(signature) => (
                StatusCode::UNAUTHORIZED,
                Json(Standard {
                    message: format!("The signature you provided [{signature}] was not correct"),
                }),
            ),
            Self::UnableToDetermineFormat => (
                StatusCode::BAD_REQUEST,
                Json(Standard {
                    message: "An image format could not be determined. Make sure the extension or the content-type header is sensible.".to_string(),
                }),
            ),
        };
        result.into_response()
    }
}

use bytes::Bytes;
use http_body_util::Full;

fn handle_panic(err: Box<dyn Any + Send + 'static>) -> Response<Full<Bytes>> {
    let details = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Unknown panic message".to_string()
    };

    let error = Standard { message: details };

    let body = serde_json::to_string(&error).expect("Could not marshal error message");

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(CONTENT_TYPE, "application/json")
        .body(Full::from(body))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use miniaturs_shared::signature::make_url_safe_base64_hash;

    use super::*;
    use std::str::FromStr;

    const SECRET: &'static str = "doyouwanttoknowasecretdoyoupromisenottotellwhoaohoh";

    #[test]
    fn test_ensure_signature_is_valid_fails_if_sig_is_wrong() -> Result<(), AppError> {
        let auth_settings = AuthenticationSettings {
            shared_secret: SECRET.to_string(),
        };

        let signature = Signature("lol".to_string());

        let uri = Uri::from_str("http://meh.com/nope")?;

        assert!(ensure_signature_is_valid(&auth_settings, &uri, signature).is_err());
        Ok(())
    }

    #[test]
    fn test_ensure_signature_is_valid_succeeds_if_sig_is_right() -> Result<(), AppError> {
        let auth_settings = AuthenticationSettings {
            shared_secret: SECRET.to_string(),
        };

        let url_string = "200x-100/https://beachape.com/images/octopress_with_container.png";

        let generated_sig = make_url_safe_base64_hash(&SECRET, url_string)?;

        let signature = Signature(generated_sig.clone());
        let uri_with_signature_and_path = format!("http://test.com/{generated_sig}/{url_string}");

        let uri = Uri::from_str(&uri_with_signature_and_path)?;

        ensure_signature_is_valid(&auth_settings, &uri, signature)
    }

    #[test]
    fn test_ensure_signature_is_valid_succeeds_if_sig_is_right_with_query() -> Result<(), AppError>
    {
        let auth_settings = AuthenticationSettings {
            shared_secret: SECRET.to_string(),
        };

        let url_string =
            "200x-100/https://beachape.com/images/octopress_with_container.png?hello=world";

        let generated_sig = make_url_safe_base64_hash(&SECRET, url_string)?;

        let signature = Signature(generated_sig.clone());
        let uri_with_signature_and_path = format!("http://test.com/{generated_sig}/{url_string}");

        let uri = Uri::from_str(&uri_with_signature_and_path)?;

        ensure_signature_is_valid(&auth_settings, &uri, signature)
    }
    #[test]
    fn test_ensure_signature_is_valid_succeeds_if_sig_is_right_with_empty_query(
    ) -> Result<(), AppError> {
        let auth_settings = AuthenticationSettings {
            shared_secret: SECRET.to_string(),
        };

        let url_string = "200x-100/https://beachape.com/images/octopress_with_container.png";

        let generated_sig = make_url_safe_base64_hash(&SECRET, url_string)?;

        let signature = Signature(generated_sig.clone());
        // Lambda + Axum
        let uri_with_signature_and_path = format!("http://test.com/{generated_sig}/{url_string}?");

        let uri = Uri::from_str(&uri_with_signature_and_path)?;
        assert!(uri.query().is_some());

        ensure_signature_is_valid(&auth_settings, &uri, signature)
    }
}
