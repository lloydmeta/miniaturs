use std::collections::HashMap;

use anyhow::Context;
use aws_sdk_s3::{
    error::DisplayErrorContext,
    primitives::{ByteStream, SdkBody},
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json;
use sha256;
use tracing::instrument;

use crate::api::requests::ImageResizePathParam;

use super::image_manipulation::Operations;

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct ImageResizeRequest {
    pub requested_image_url: String,
    pub operations: Operations,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone, Copy)]
pub struct ImageResize {
    pub target_width: i32,
    pub target_height: i32,
}

impl From<ImageResizePathParam> for ImageResize {
    fn from(
        ImageResizePathParam {
            target_width,
            target_height,
        }: ImageResizePathParam,
    ) -> Self {
        Self {
            target_width,
            target_height,
        }
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct ImageResizedCacheRequest {
    pub request: ImageResizeRequest,
    pub content_type: String,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct ImageFetchRequest {
    pub requested_image_url: String,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct ImageFetchedCacheRequest {
    pub request: ImageFetchRequest,
    pub content_type: Option<String>,
}

pub struct Retrieved<CacheRequest> {
    pub bytes: Vec<u8>,
    pub requested: CacheRequest,
}

#[allow(async_fn_in_trait)]
pub trait ImageCacher<GetRequest, SetRequest>
where
    GetRequest: CacheGettable<Cached = SetRequest>,
    SetRequest: CacheSettable<Retrieve = GetRequest>,
{
    async fn get(&self, req: &GetRequest) -> anyhow::Result<Option<Retrieved<SetRequest>>>;
    async fn set(&self, bytes: &[u8], req: &SetRequest) -> anyhow::Result<()>;
}

pub trait CacheGettable {
    type Cached;
    fn cache_key(&self) -> anyhow::Result<CacheKey>;
}
pub trait CacheSettable: CacheGettable {
    type Retrieve;
    fn metadata(&self) -> anyhow::Result<Metadata>;
}

#[derive(Debug, Clone)]
pub struct S3ImageCacher {
    client: aws_sdk_s3::Client,
    bucket_name: String,
}

impl S3ImageCacher {
    pub fn new(client: aws_sdk_s3::Client, bucket_name: &str) -> Self {
        S3ImageCacher {
            client,
            bucket_name: bucket_name.to_string(),
        }
    }
}

impl<GetReq, SetReq> ImageCacher<GetReq, SetReq> for S3ImageCacher
where
    GetReq: CacheGettable<Cached = SetReq> + std::fmt::Debug,
    SetReq: CacheSettable<Retrieve = GetReq> + DeserializeOwned + std::fmt::Debug,
{
    #[instrument]
    async fn get(&self, req: &GetReq) -> anyhow::Result<Option<Retrieved<SetReq>>> {
        let cache_key = req.cache_key()?;

        let cache_retrieve_attempt = self
            .client
            .get_object()
            .bucket(self.bucket_name.to_owned())
            .key(cache_key.0)
            .send()
            .await;
        match cache_retrieve_attempt {
            Ok(retreived) => {
                // If we can't retrieve metadata, it's dead to us !
                let maybe_s3_metadata = retreived.metadata().and_then(|m| m.get(METADATA_JSON_KEY));
                let maybe_as_original_requested =
                    maybe_s3_metadata.and_then(|m| serde_json::from_str(m).ok());

                Ok(match maybe_as_original_requested {
                    Some(as_original_requested) => {
                        let bytes: Vec<u8> = retreived
                            .body
                            .collect()
                            .await
                            .map_err(|e| anyhow::anyhow!("Failure to retrieve from S3 [{e}]"))?
                            .to_vec();
                        Some(Retrieved {
                            bytes,
                            requested: as_original_requested,
                        })
                    }
                    None => None,
                })
            }
            Err(sdk_err)
                if sdk_err
                    .as_service_error()
                    .filter(|e| e.is_no_such_key())
                    .is_some() =>
            {
                Ok(None)
            }
            // Anything else is fucked.
            Err(other) => anyhow::bail!("AWS S3 SDK error: [{}]", DisplayErrorContext(other)),
        }
    }

    #[instrument(skip(bytes))]
    async fn set(&self, bytes: &[u8], req: &SetReq) -> anyhow::Result<()> {
        let body_stream = ByteStream::new(SdkBody::from(bytes));

        let metadata = req.metadata()?;
        let cache_key = req.cache_key()?;

        self.client
            .put_object()
            .bucket(self.bucket_name.clone())
            .key(cache_key.0)
            .set_metadata(Some(metadata.0))
            .body(body_stream)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Writing to S3 failed [{e}]"))?;
        Ok(())
    }
}

pub struct Metadata(HashMap<String, String>);
#[derive(Debug)]
pub struct CacheKey(String);

static METADATA_JSON_KEY: &str = "_metadata_json";
impl CacheGettable for ImageResizeRequest {
    type Cached = ImageResizedCacheRequest;
    fn cache_key(&self) -> anyhow::Result<CacheKey> {
        let as_json = serde_json::to_string(self).context("Could not JSON-ify to cache key.")?;
        let sha256ed = sha256::digest(as_json);
        Ok(CacheKey(sha256ed))
    }
}
impl CacheGettable for ImageResizedCacheRequest {
    type Cached = Self;
    fn cache_key(&self) -> anyhow::Result<CacheKey> {
        self.request.cache_key()
    }
}
impl CacheSettable for ImageResizedCacheRequest {
    type Retrieve = ImageResizeRequest;
    fn metadata(&self) -> anyhow::Result<Metadata> {
        let as_json_string =
            serde_json::to_string(self).context("Could not JSON-ify to metadata.")?;
        let mut map = HashMap::new();
        map.insert(METADATA_JSON_KEY.to_string(), as_json_string);
        Ok(Metadata(map))
    }
}

impl CacheGettable for ImageFetchRequest {
    type Cached = ImageFetchedCacheRequest;
    fn cache_key(&self) -> anyhow::Result<CacheKey> {
        let as_json = serde_json::to_string(self).context("Could not JSON-ify to cache key.")?;
        let sha256ed = sha256::digest(as_json);
        Ok(CacheKey(sha256ed))
    }
}
impl CacheGettable for ImageFetchedCacheRequest {
    type Cached = Self;
    fn cache_key(&self) -> anyhow::Result<CacheKey> {
        self.request.cache_key()
    }
}
impl CacheSettable for ImageFetchedCacheRequest {
    type Retrieve = ImageFetchRequest;
    fn metadata(&self) -> anyhow::Result<Metadata> {
        let as_json_string =
            serde_json::to_string(self).context("Could not JSON-ify to metadata.")?;
        let mut map = HashMap::new();
        map.insert(METADATA_JSON_KEY.to_string(), as_json_string);
        Ok(Metadata(map))
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use aws_sdk_s3::primitives::ByteStream;
    use tokio::sync::OnceCell;

    use super::*;
    use crate::test_utils::{s3_client, TestResult};

    // Bucket, static because we assume the app is passed a created one.
    static S3_BUCKET: OnceCell<String> = OnceCell::const_new();

    #[tokio::test]
    async fn test_s3() -> TestResult<()> {
        let client = s3_client().await;

        let bucket = "mybucket";
        let key = "my_key";
        let content = b"testcontent";
        let mut metadata = HashMap::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        metadata.insert("key2".to_string(), "value2".to_string());

        let body_stream = ByteStream::from_static(content);

        client.create_bucket().bucket(bucket).send().await?;

        client
            .put_object()
            .bucket(bucket)
            .key(key)
            .set_metadata(Some(metadata.clone()))
            .body(body_stream)
            .send()
            .await?;

        let read_back = client.get_object().bucket(bucket).key(key).send().await?;

        let read_back_metadata = read_back.metadata().unwrap();
        assert_eq!(&metadata, read_back_metadata);

        let body_bytes = read_back.body.collect().await?.into_bytes();
        let body_bytes_ref = body_bytes.as_ref();

        assert_eq!(content, body_bytes_ref);

        Ok(())
    }

    #[test]
    fn test_cache_key() -> TestResult<()> {
        let req = ImageResizeRequest {
            requested_image_url: "https://beachape.com/images/something.png".to_string(),
            operations: Operations::build(&Some(ImageResize {
                target_width: 100,
                target_height: 100,
            })),
        };
        let key = req.cache_key()?;
        assert!(key.0.len() < 1024);
        Ok(())
    }

    #[test]
    fn test_metadata() -> TestResult<()> {
        let req = ImageResizedCacheRequest {
            request: ImageResizeRequest {
                requested_image_url: "https://beachape.com/images/something.png".to_string(),
                operations: Operations::build(&Some(ImageResize {
                    target_width: 100,
                    target_height: 200,
                })),
            },
            content_type: "image/png".to_string(),
        };
        let metadata = req.metadata()?;

        let mut expected = HashMap::new();
        let metadata_as_json = serde_json::to_string(&req).unwrap();

        expected.insert(METADATA_JSON_KEY.to_string(), metadata_as_json);
        assert_eq!(expected, metadata.0);
        Ok(())
    }

    #[tokio::test]
    async fn test_s3_image_cacher_get_does_not_exist() -> TestResult<()> {
        let client = s3_client().await.clone();
        let bucket_name = s3_bucket().await.clone();
        let s3_image_cacher = S3ImageCacher {
            client,
            bucket_name,
        };
        let req = ImageResizeRequest {
            requested_image_url: "https://beachape.com/images/something_that_does_not_exist.png"
                .to_string(),
            operations: Operations::build(&Some(ImageResize {
                target_width: 100,
                target_height: 100,
            })),
        };
        let retrieved = s3_image_cacher.get(&req).await;
        assert!(retrieved?.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_s3_image_cacher_set_get() -> TestResult<()> {
        let client = s3_client().await.clone();
        let bucket_name = s3_bucket().await.clone();
        let s3_image_cacher = S3ImageCacher {
            client,
            bucket_name,
        };

        let req = ImageResizeRequest {
            requested_image_url: "https://beachape.com/images/something.png".to_string(),
            operations: Operations::build(&Some(ImageResize {
                target_width: 100,
                target_height: 100,
            })),
        };
        let content = b"testcontent";
        let image_set_req = ImageResizedCacheRequest {
            request: req.clone(),
            content_type: "image/png".to_string(),
        };
        s3_image_cacher.set(content, &image_set_req).await?;
        let retrieved = s3_image_cacher
            .get(&req)
            .await?
            .expect("Cached image should be retrievable");
        assert_eq!(content, retrieved.bytes.as_slice());
        assert_eq!(image_set_req, retrieved.requested);
        Ok(())
    }

    #[tokio::test]
    async fn test_s3_image_cacher_setting_metadata_in_s3() -> TestResult<()> {
        let client = s3_client().await.clone();
        let bucket_name = s3_bucket().await.clone();
        let s3_image_cacher = S3ImageCacher {
            client,
            bucket_name,
        };
        let req = ImageResizeRequest {
            requested_image_url: "https://beachape.com/images/something_else.png".to_string(),
            operations: Operations::build(&Some(ImageResize {
                target_width: 300,
                target_height: 500,
            })),
        };
        let content = b"testcontent";
        let image_set_req = ImageResizedCacheRequest {
            request: req.clone(),
            content_type: "image/png".to_string(),
        };
        s3_image_cacher.set(content, &image_set_req).await?;

        let cache_key = req.cache_key()?;
        let metadata_from_req = image_set_req.metadata()?;

        let retrieved = s3_client()
            .await
            .get_object()
            .bucket(s3_bucket().await.clone())
            .key(cache_key.0)
            .send()
            .await?;

        let retrieved_metadata = retrieved
            .metadata()
            .map(|m| m.to_owned())
            .expect("There should be metadata");

        assert_eq!(metadata_from_req.0, retrieved_metadata);
        Ok(())
    }

    static BUCKET_NAME: &'static str = "my-test-bucket";
    async fn s3_bucket() -> &'static String {
        S3_BUCKET
            .get_or_init(|| async {
                s3_client()
                    .await
                    .create_bucket()
                    .bucket(BUCKET_NAME.to_string())
                    .send()
                    .await
                    .expect("Bucket creation should work");
                BUCKET_NAME.to_string()
            })
            .await
    }
}
