use serde::*;

use crate::infra::image_manipulation;

#[derive(Serialize)]
pub struct Standard {
    pub message: String,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct MetadataResponse {
    pub source: Source,
    pub operations: Vec<Operation>,
}

impl MetadataResponse {
    pub fn build(url: &str, ops: &image_manipulation::Operations) -> Self {
        let operations = ops
            .0
            .iter()
            .map(|op| match *op {
                image_manipulation::Operation::Resize { width, height } => Operation {
                    r#type: "resize".to_string(),
                    width: Some(width),
                    height: Some(height),
                },
                image_manipulation::Operation::FlipHorizontally => Operation {
                    r#type: "flip_horizontally".to_string(),
                    width: None,
                    height: None,
                },
                image_manipulation::Operation::FlipVertically => Operation {
                    r#type: "flip_vertically".to_string(),
                    width: None,
                    height: None,
                },
            })
            .collect();

        MetadataResponse {
            source: Source {
                url: url.to_string(),
            },
            operations,
        }
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct Source {
    pub url: String,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct Operation {
    pub r#type: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[cfg(test)]
mod tests {
    use crate::infra::image_caching::ImageResize;

    use super::*;

    #[test]
    fn test_metadata_response_build() {
        let domain = image_manipulation::Operations::build(&Some(ImageResize {
            target_width: -100,
            target_height: -300,
        }));
        let result = MetadataResponse::build("http://beachape.com/images/lol.png", &domain);
        let expected = MetadataResponse {
            source: Source {
                url: "http://beachape.com/images/lol.png".to_string(),
            },
            operations: vec![
                Operation {
                    r#type: "resize".to_string(),
                    width: Some(100),
                    height: Some(300),
                },
                Operation {
                    r#type: "flip_horizontally".to_string(),
                    width: None,
                    height: None,
                },
                Operation {
                    r#type: "flip_vertically".to_string(),
                    width: None,
                    height: None,
                },
            ],
        };
        assert_eq!(expected, result)
    }
}
