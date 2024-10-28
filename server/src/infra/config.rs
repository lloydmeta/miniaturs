use std::env;

use anyhow::Context;
use aws_config::{BehaviorVersion, SdkConfig};

const SHARED_SECRET_ENV_KEY: &str = "MINIATURS_SHARED_SECRET";
const PROCESSED_IMAGES_BUCKET_NAME_ENV_KEY: &str = "PROCESSED_IMAGES_BUCKET";
const UNPROCESSED_IMAGES_BUCKET_NAME_ENV_KEY: &str = "UNPROCESSED_IMAGES_BUCKET";
const REQUIRE_PATH_STYLE_S3_KEY: &str = "REQUIRE_PATH_STYLE_S3";

#[derive(Clone)]
pub struct Config {
    pub authentication_settings: AuthenticationSettings,
    pub image_cache_settings: ImageCacheSettings,
    pub aws_settings: AwsSettings,
}

#[derive(Clone)]
pub struct AuthenticationSettings {
    pub shared_secret: String,
}

#[derive(Clone)]
pub struct ImageCacheSettings {
    pub processed_images_bucket_name: String,
    pub unprocessed_images_bucket_name: String,
}

#[derive(Clone)]
pub struct AwsSettings {
    pub aws_config: SdkConfig,
    pub path_style_s3: bool,
}

impl Config {
    pub async fn load_env() -> Result<Config, anyhow::Error> {
        let shared_secret = env::var(SHARED_SECRET_ENV_KEY)
            .context("Expected {SHARED_SECRET_ENV_KEY} to be defined")?;

        let authentication_settings = AuthenticationSettings { shared_secret };

        let processed_images_bucket_name = env::var(PROCESSED_IMAGES_BUCKET_NAME_ENV_KEY)
            .context("Expected {PROCESSED_IMAGES_BUCKET_NAME_ENV_KEY} to be defined")?;
        let unprocessed_images_bucket_name = env::var(UNPROCESSED_IMAGES_BUCKET_NAME_ENV_KEY)
            .context("Expected {UNPROCESSED_IMAGES_BUCKET_NAME_ENV_KEY} to be defined")?;

        let image_cache_settings = ImageCacheSettings {
            processed_images_bucket_name,
            unprocessed_images_bucket_name,
        };

        let path_style_s3 = env::var(REQUIRE_PATH_STYLE_S3_KEY)
            .ok()
            .and_then(|s| str::parse::<bool>(&s).ok())
            .is_some();
        let aws_config = aws_config::load_defaults(BehaviorVersion::v2024_03_28()).await;
        let aws_settings = AwsSettings {
            aws_config,
            path_style_s3,
        };

        Ok(Config {
            authentication_settings,
            image_cache_settings,
            aws_settings,
        })
    }
}
