use image::DynamicImage;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::image_caching::ImageResize;

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Operation {
    Resize { width: u32, height: u32 },
    FlipHorizontally,
    FlipVertically,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize, Clone)]
pub struct Operations(pub Vec<Operation>);

impl Operations {
    pub fn build(image_resize: &Option<ImageResize>) -> Self {
        let mut v = Vec::new();

        if let Some(image_resize) = image_resize {
            v.push(Operation::Resize {
                width: image_resize.target_width.unsigned_abs(),
                height: image_resize.target_height.unsigned_abs(),
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

#[derive(Debug)]
pub struct SingletonOperationsRunner;

impl OperationsRunner for SingletonOperationsRunner {
    #[instrument(skip(image))]
    async fn run(&self, image: DynamicImage, operations: &Operations) -> DynamicImage {
        operations.0.iter().fold(image, |next, op| match op {
            Operation::Resize { width, height } => {
                let resize_to_width = if *width == 0 { next.width() } else { *width };
                let resize_to_height = if *height == 0 { next.height() } else { *height };
                next.resize(
                    resize_to_width,
                    resize_to_height,
                    image::imageops::FilterType::Lanczos3,
                )
            }
            Operation::FlipHorizontally => next.fliph(),
            Operation::FlipVertically => next.flipv(),
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

    #[tokio::test]
    async fn test_operations_runner_no_resize() {
        let image_bin = include_bytes!("not-aliens.jpg");
        let image_reader = ImageReader::new(Cursor::new(image_bin))
            .with_guessed_format()
            .unwrap();

        let image = image_reader.decode().unwrap();
        let original_image = image.clone();

        let operations = Operations::build(&Some(ImageResize {
            target_width: 0,
            target_height: 0,
        }));

        let result = SingletonOperationsRunner.run(image, &operations).await;
        assert_eq!(original_image.width(), result.width());
        assert_eq!(original_image.height(), result.height());
    }
}
