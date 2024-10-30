use image::DynamicImage;
use serde::{Deserialize, Serialize};

use super::image_caching::ImageResize;

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Operation {
    Resize { width: u32, height: u32 },
    FlipHorizontally,
    FlipVertically,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Operations(pub Vec<Operation>);

impl Operations {
    pub fn build(image_resize: &Option<ImageResize>) -> Self {
        let mut v = Vec::new();

        if let Some(image_resize) = image_resize {
            v.push(Operation::Resize {
                width: image_resize.target_width.abs() as u32,
                height: image_resize.target_height.abs() as u32,
            });
            if image_resize.target_width.is_negative() {
                v.push(Operation::FlipHorizontally);
            }
            if image_resize.target_height.is_negative() {
                v.push(Operation::FlipVertically);
            }
        }

        Operations(v)
    }
}

// Runner of operations
// Async in case we need to go multi-threaded
#[allow(async_fn_in_trait)]
pub trait OperationsRunner {
    async fn run(&self, image: DynamicImage, operations: &Operations) -> DynamicImage;
}

pub struct SingletonOperationsRunner;

impl OperationsRunner for SingletonOperationsRunner {
    async fn run(&self, image: DynamicImage, operations: &Operations) -> DynamicImage {
        operations.0.iter().fold(image, |mut next, op| {
            next = match op {
                Operation::Resize { width, height } => {
                    next.resize(*width, *height, image::imageops::FilterType::Gaussian)
                }
                Operation::FlipHorizontally => next.fliph(),
                Operation::FlipVertically => next.flipv(),
            };
            next
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::ImageReader;

    use super::*;

    #[test]
    fn test_operations_build_nothing() {
        let r = Operations::build(&None);
        assert!(r.0.is_empty());
    }

    #[test]
    fn test_operations_build_resize() {
        let r = Operations::build(&Some(ImageResize {
            target_width: 1,
            target_height: 2,
        }));
        assert_eq!(
            Operation::Resize {
                width: 1,
                height: 2
            },
            r.0[0]
        );
    }

    #[test]
    fn test_operations_build_resize_with_flips() {
        let r = Operations::build(&Some(ImageResize {
            target_width: -3,
            target_height: -4,
        }));
        assert_eq!(
            Operation::Resize {
                width: 3,
                height: 4
            },
            r.0[0]
        );
        assert_eq!(Operation::FlipHorizontally, r.0[1]);
        assert_eq!(Operation::FlipVertically, r.0[2]);
    }

    #[tokio::test]
    async fn test_operations_runner() {
        let image_bin = include_bytes!("not-aliens.jpg");
        let image_reader = ImageReader::new(Cursor::new(image_bin))
            .with_guessed_format()
            .unwrap();

        let image = image_reader.decode().unwrap();
        let operations = Operations::build(&Some(ImageResize {
            target_width: -3,
            target_height: -4,
        }));

        let result = SingletonOperationsRunner.run(image, &operations).await;
        assert_eq!(3, result.width());
        assert_eq!(2, result.height());
    }
}
