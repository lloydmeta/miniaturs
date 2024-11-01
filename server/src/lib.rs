pub mod api;
pub mod infra;

#[cfg(test)]
pub mod test_utils;

#[cfg(test)]
mod integration_tests {
    use std::io::Cursor;

    use crate::api::routing::handlers::create_router;
    use crate::infra::components::AppComponents;
    use crate::infra::config::Config;
    use crate::test_utils::{localstack_node, s3_client, TestResult};

    use super::api::responses::MetadataResponse;
    use super::infra::config::{AuthenticationSettings, AwsSettings, ImageCacheSettings};
    use super::infra::image_caching::*;
    use super::infra::image_manipulation::Operations;
    use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
    use axum::{body::Body, http::Request, Router};
    use http_body_util::BodyExt;
    use image::{ImageFormat, ImageReader};
    use lambda_http::tower::ServiceExt;
    use miniaturs_shared::signature::make_url_safe_base64_hash;
    use reqwest::{header::CONTENT_TYPE, StatusCode};
    use tokio::sync::OnceCell;

    static UNPROCESSED_BUCKET: OnceCell<String> = OnceCell::const_new();
    static PROCESSED_BUCKET: OnceCell<String> = OnceCell::const_new();
    static CONFIG: OnceCell<Config> = OnceCell::const_new();
    static AWS_CONFIG: OnceCell<aws_config::SdkConfig> = OnceCell::const_new();

    #[tokio::test]
    async fn test_root_response() -> TestResult<()> {
        let app = app().await?;
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty())?)
            .await?;
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    const PNG_URL_1: &'static str = "https://beachape.com/images/octopress_with_container.png";
    const JPG_URL_1: &'static str = "https://beachape.com/images/super-high-performance.jpg";

    #[tokio::test]
    async fn test_resize_png() -> TestResult<()> {
        test_resize(
            PNG_URL_1,
            ImageFormat::Png,
            ImageResize {
                target_width: 100,
                target_height: 80,
            },
            ImageResize {
                target_width: 300,
                target_height: 100,
            },
        )
        .await
    }

    #[tokio::test]
    async fn test_resize_jpg() -> TestResult<()> {
        test_resize(
            JPG_URL_1,
            ImageFormat::Jpeg,
            ImageResize {
                target_width: 50,
                target_height: 70,
            },
            ImageResize {
                target_width: 500,
                target_height: 600,
            },
        )
        .await
    }

    #[tokio::test]
    async fn test_metadata_response() -> TestResult<()> {
        test_metadata(
            &PNG_URL_1,
            ImageResize {
                target_width: 100,
                target_height: 80,
            },
        )
        .await?;
        test_metadata(
            &PNG_URL_1,
            ImageResize {
                target_width: -100,
                target_height: 80,
            },
        )
        .await?;
        test_metadata(
            &JPG_URL_1,
            ImageResize {
                target_width: 100,
                target_height: -80,
            },
        )
        .await?;
        test_metadata(
            &PNG_URL_1,
            ImageResize {
                target_width: -100,
                target_height: -80,
            },
        )
        .await
    }

    async fn test_resize(
        image_url: &str,
        expected_image_format: ImageFormat,
        resize_target_1: ImageResize,
        resize_target_2: ImageResize,
    ) -> TestResult<()> {
        let target_width_1 = resize_target_1.target_width;

        let signed_path_1 = signed_resize_path(
            &config().await.authentication_settings,
            resize_target_1,
            image_url,
        )?;

        // ensure nothing cached right now
        assert!(retrieve_unprocessed_cached(image_url).await.is_none());
        assert!(retrieve_processed_cached(image_url, resize_target_1)
            .await
            .is_none());
        assert!(retrieve_processed_cached(image_url, resize_target_2)
            .await
            .is_none());

        let response_1 = app()
            .await?
            .oneshot(Request::builder().uri(signed_path_1).body(Body::empty())?)
            .await?;
        assert_eq!(StatusCode::OK, response_1.status());

        // ensure what should be cached is cached
        assert!(retrieve_unprocessed_cached(image_url).await.is_some());
        assert!(retrieve_processed_cached(image_url, resize_target_1)
            .await
            .is_some());
        assert!(retrieve_processed_cached(image_url, resize_target_2)
            .await
            .is_none());

        let response_content_type_1 = response_1
            .headers()
            .get(CONTENT_TYPE)
            .ok_or("No content type in response from miniaturs")?
            .to_str()?;
        assert_eq!(
            expected_image_format.to_mime_type(),
            response_content_type_1
        );

        let response_bytes_1 = response_1.into_body().collect().await?.to_bytes();

        let image_reader_1 =
            ImageReader::new(Cursor::new(response_bytes_1)).with_guessed_format()?;

        assert_eq!(
            expected_image_format,
            image_reader_1.format().ok_or("Could not guess format")?
        );
        let dynamic_image_1 = image_reader_1.decode()?;

        assert_eq!(target_width_1 as u32, dynamic_image_1.width());

        // resize again
        let target_width_2 = resize_target_2.target_width;
        let signed_path_2 = signed_resize_path(
            &config().await.authentication_settings,
            resize_target_2,
            image_url,
        )?;

        let response_2 = app()
            .await?
            .oneshot(Request::builder().uri(signed_path_2).body(Body::empty())?)
            .await?;
        assert_eq!(StatusCode::OK, response_2.status());
        // ensure what should be cached is cached
        assert!(retrieve_unprocessed_cached(image_url).await.is_some());
        assert!(retrieve_processed_cached(image_url, resize_target_1)
            .await
            .is_some());
        assert!(retrieve_processed_cached(image_url, resize_target_2)
            .await
            .is_some());

        let response_content_type_2 = response_2
            .headers()
            .get(CONTENT_TYPE)
            .ok_or("No content type in response from miniaturs")?
            .to_str()?;
        assert_eq!(
            expected_image_format.to_mime_type(),
            response_content_type_2
        );

        let response_bytes_2 = response_2.into_body().collect().await?.to_bytes();

        let image_reader_2 =
            ImageReader::new(Cursor::new(response_bytes_2)).with_guessed_format()?;

        assert_eq!(
            expected_image_format,
            image_reader_2.format().ok_or("Could not guess format")?
        );
        let dynamic_image_2 = image_reader_2.decode()?;

        assert_eq!(target_width_2 as u32, dynamic_image_2.width());

        Ok(())
    }

    async fn test_metadata(image_url: &str, resize: ImageResize) -> TestResult<()> {
        let signed_path =
            signed_metadata_path(&config().await.authentication_settings, resize, image_url)?;
        let response = app()
            .await?
            .oneshot(Request::builder().uri(signed_path).body(Body::empty())?)
            .await?;
        assert_eq!(StatusCode::OK, response.status());

        let mut body_as_metadata: MetadataResponse =
            serde_json::from_slice(response.into_body().collect().await?.to_bytes().as_ref())?;

        assert_eq!(image_url, body_as_metadata.source.url);

        // So we can pop easily
        body_as_metadata.operations.reverse();

        let mut op = body_as_metadata.operations.pop().unwrap();
        assert_eq!("resize", op.r#type);
        assert_eq!(resize.target_width.unsigned_abs(), op.width.unwrap());
        assert_eq!(resize.target_height.unsigned_abs(), op.height.unwrap());

        if resize.target_width.is_negative() {
            op = body_as_metadata.operations.pop().unwrap();
            assert_eq!("flip_horizontally", op.r#type);
            assert_eq!(None, op.width);
            assert_eq!(None, op.height);
        }

        if resize.target_height.is_negative() {
            op = body_as_metadata.operations.pop().unwrap();
            assert_eq!("flip_vertically", op.r#type);
            assert_eq!(None, op.width);
            assert_eq!(None, op.height);
        }

        Ok(())
    }

    async fn retrieve_unprocessed_cached(
        image_url: &str,
    ) -> Option<Retrieved<ImageFetchedCacheRequest>> {
        let config = config().await;
        let app_components = AppComponents::create(config.clone()).ok()?;
        let unprocessed_cache_retrieve_req = ImageFetchRequest {
            requested_image_url: image_url.to_string(),
        };
        app_components
            .unprocessed_images_cacher
            .get(&unprocessed_cache_retrieve_req)
            .await
            .unwrap()
    }

    async fn retrieve_processed_cached(
        image_url: &str,
        resize_target: ImageResize,
    ) -> Option<Retrieved<ImageResizedCacheRequest>> {
        let config = config().await;
        let app_components = AppComponents::create(config.clone()).ok()?;
        let processed_cache_retrieve_req = ImageResizeRequest {
            requested_image_url: image_url.to_string(),
            operations: Operations::build(&Some(resize_target)),
        };
        app_components
            .processed_images_cacher
            .get(&processed_cache_retrieve_req)
            .await
            .unwrap()
    }

    fn signed_resize_path(
        auth_settings: &AuthenticationSettings,
        resize_target: ImageResize,

        url: &str,
    ) -> TestResult<String> {
        let target_width = resize_target.target_width;
        let target_height = resize_target.target_height;
        let path = format!("{target_width}x{target_height}/{url}");
        let hash = make_url_safe_base64_hash(&auth_settings.shared_secret, &path)?;
        Ok(format!("/{hash}/{path}"))
    }

    fn signed_metadata_path(
        auth_settings: &AuthenticationSettings,
        resize_target: ImageResize,

        url: &str,
    ) -> TestResult<String> {
        let target_width = resize_target.target_width;
        let target_height = resize_target.target_height;
        let path = format!("meta/{target_width}x{target_height}/{url}");
        let hash = make_url_safe_base64_hash(&auth_settings.shared_secret, &path)?;
        Ok(format!("/{hash}/{path}"))
    }

    async fn app() -> Result<Router, Box<dyn std::error::Error + 'static>> {
        let config = config().await;
        let app_components = AppComponents::create(config.clone())?;
        Ok(create_router(app_components))
    }

    async fn config() -> &'static Config {
        CONFIG
            .get_or_init(|| async {
                let authentication_settings = AuthenticationSettings {
                    shared_secret: "omgwtfbbq".to_string(),
                };

                let image_cache_settings = ImageCacheSettings {
                    processed_images_bucket_name: processed_bucket().await.to_string(),
                    unprocessed_images_bucket_name: unprocessed_bucket().await.to_string(),
                };

                let aws_settings = AwsSettings {
                    aws_config: aws_config().await.clone(),
                    path_style_s3: true,
                };

                Config {
                    authentication_settings,
                    image_cache_settings,
                    aws_settings,
                }
            })
            .await
    }

    static UNPROCCESSED_BUCKET_NAME: &'static str = "unprocessed-bucket";
    async fn unprocessed_bucket() -> &'static String {
        UNPROCESSED_BUCKET
            .get_or_init(|| async {
                bootstrap_s3_client()
                    .await
                    .create_bucket()
                    .bucket(UNPROCCESSED_BUCKET_NAME.to_string())
                    .send()
                    .await
                    .expect("Bucket creation should work");
                UNPROCCESSED_BUCKET_NAME.to_string()
            })
            .await
    }
    static PROCCESSED_BUCKET_NAME: &'static str = "processed-bucket";
    async fn processed_bucket() -> &'static String {
        PROCESSED_BUCKET
            .get_or_init(|| async {
                bootstrap_s3_client()
                    .await
                    .create_bucket()
                    .bucket(PROCCESSED_BUCKET_NAME.to_string())
                    .send()
                    .await
                    .expect("Bucket creation should work");
                PROCCESSED_BUCKET_NAME.to_string()
            })
            .await
    }

    async fn bootstrap_s3_client() -> &'static aws_sdk_s3::Client {
        s3_client().await
    }

    async fn aws_config() -> &'static aws_config::SdkConfig {
        AWS_CONFIG
            .get_or_init(|| async {
                let node = localstack_node().await;
                let host_port = node
                    .get_host_port_ipv4(4566)
                    .await
                    .expect("Port from Localstack to be retrievable");

                let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
                let region = region_provider.region().await.unwrap();
                let creds =
                    aws_sdk_s3::config::Credentials::new("fake", "fake", None, None, "test");
                aws_config::defaults(BehaviorVersion::v2024_03_28())
                    .region(region.clone())
                    .credentials_provider(creds)
                    .endpoint_url(format!("http://127.0.0.1:{host_port}"))
                    .load()
                    .await
            })
            .await
    }
}
