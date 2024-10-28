use anyhow::Error;
use reqwest::Client;

use super::{
    config::{AwsSettings, Config},
    image_caching::S3ImageCacher,
};

#[derive(Clone)]
pub struct AppComponents {
    pub http_client: reqwest::Client,
    pub config: Config,
    pub processed_images_cacher: S3ImageCacher,
    pub unprocessed_images_cacher: S3ImageCacher,
}

impl AppComponents {
    pub fn create(config: Config) -> Result<AppComponents, Error> {
        let s3_client = s3_client(&config.aws_settings);
        let processed_images_cacher = S3ImageCacher::new(
            s3_client.clone(),
            &config.image_cache_settings.processed_images_bucket_name,
        );
        let unprocessed_images_cacher = S3ImageCacher::new(
            s3_client.clone(),
            &config.image_cache_settings.unprocessed_images_bucket_name,
        );

        let client = Client::new();
        Ok(AppComponents {
            http_client: client,
            config,
            processed_images_cacher,
            unprocessed_images_cacher,
        })
    }
}

fn s3_client(settings: &AwsSettings) -> aws_sdk_s3::Client {
    let mut s3_config_builder = aws_sdk_s3::config::Builder::from(&settings.aws_config);
    // Prevents DNS errors if using localstack connecting from
    // host machine
    if settings.path_style_s3 {
        s3_config_builder.set_force_path_style(Some(true));
    }

    aws_sdk_s3::Client::from_conf(s3_config_builder.build())
}
