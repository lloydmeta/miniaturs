use std::env;

use anyhow::Context;
use aws_config::{BehaviorVersion, SdkConfig};

const SHARED_SECRET_ENV_KEY: &str = "MINIATURS_SHARED_SECRET";
const PROCESSED_IMAGES_BUCKET_NAME_ENV_KEY: &str = "PROCESSED_IMAGES_BUCKET";
const UNPROCESSED_IMAGES_BUCKET_NAME_ENV_KEY: &str = "UNPROCESSED_IMAGES_BUCKET";
const REQUIRE_PATH_STYLE_S3_KEY: &str = "REQUIRE_PATH_STYLE_S3";
const MAX_RESIZE_TARGET_WIDTH: &str = "MAX_RESIZE_TARGET_WIDTH";
const MAX_RESIZE_TARGET_HEIGHT: &str = "MAX_RESIZE_TARGET_HEIGHT";
const MAX_SOURCE_TARGET_WIDTH: &str = "MAX_SOURCE_TARGET_WIDTH";
const MAX_SOURCE_TARGET_HEIGHT: &str = "MAX_SOURCE_TARGET_HEIGHT";

#[derive(Clone)]
pub struct Config {
    pub authentication_settings: AuthenticationSettings,
    pub image_cache_settings: ImageCacheSettings,
    pub aws_settings: AwsSettings,
    pub validation_settings: ValidationSettings,
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

#[derive(Clone)]
pub struct ValidationSettings {
    // Max width that we will resize to (pixels)
    pub max_resize_target_width: u32,
    // Max height that we will resize to (pixels)
    pub max_resize_target_height: u32,
    // Max width the source image can have (pixels)
    pub max_source_image_width: u32,
    // Max height the source image can have (pixels)
    pub max_source_image_height: u32,
}

static MAX_PIXELS_DEFAULT: u32 = 10000;

impl Default for ValidationSettings {
    fn default() -> Self {
        Self {
            max_resize_target_width: MAX_PIXELS_DEFAULT,
            max_resize_target_height: MAX_PIXELS_DEFAULT,
            max_source_image_width: MAX_PIXELS_DEFAULT,
            max_source_image_height: MAX_PIXELS_DEFAULT,
        }
    }
}

impl Config {
    pub async fn load_env() -> anyhow::Result<Config> {
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

        let mut validation_settings = ValidationSettings::default();

        if let Some(max_resize_target_width) = env::var(MAX_RESIZE_TARGET_WIDTH)
            .ok()
            .and_then(|s| s.parse().ok())
        {
            validation_settings.max_resize_target_width = max_resize_target_width;
        }
        if let Some(max_resize_target_height) = env::var(MAX_RESIZE_TARGET_HEIGHT)
            .ok()
            .and_then(|s| s.parse().ok())
        {
            validation_settings.max_resize_target_height = max_resize_target_height;
        }
        if let Some(max_source_image_width) = env::var(MAX_SOURCE_TARGET_WIDTH)
            .ok()
            .and_then(|s| s.parse().ok())
        {
            validation_settings.max_source_image_width = max_source_image_width;
        }
        if let Some(max_source_image_height) = env::var(MAX_SOURCE_TARGET_HEIGHT)
            .ok()
            .and_then(|s| s.parse().ok())
        {
            validation_settings.max_source_image_height = max_source_image_height;
        }

        Ok(Config {
            authentication_settings,
            image_cache_settings,
            aws_settings,
            validation_settings,
        })
    }
}
