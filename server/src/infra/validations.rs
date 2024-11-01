use image::DynamicImage;

use super::{config::ValidationSettings, image_manipulation::Operations};

pub trait Validator {
    fn validate_operations(
        &self,
        settings: &ValidationSettings,
        operations: &Operations,
    ) -> Result<(), ValidationErrors>;

    fn validate_source_image(
        &self,
        settings: &ValidationSettings,
        image: &DynamicImage,
    ) -> Result<(), ValidationErrors>;
}

pub struct SimpleValidator;

pub struct ValidationErrors(pub Vec<String>);

impl Validator for SimpleValidator {
    fn validate_operations(
        &self,
        settings: &ValidationSettings,
        operations: &Operations,
    ) -> Result<(), ValidationErrors> {
        let problems = operations
            .0
            .iter()
            .fold(Vec::new(), |mut next, op| match *op {
                crate::infra::image_manipulation::Operation::Resize { width, height } => {
                    if width > settings.max_resize_target_width {
                        next.push(format!(
                            "Resize target width [{width}] too large, must be [{}] or lower",
                            settings.max_resize_target_width
                        ));
                    }
                    if height > settings.max_resize_target_height {
                        next.push(format!(
                            "Resize target height [{height}] too large, must be [{}] or lower",
                            settings.max_resize_target_height
                        ));
                    }
                    next
                }
                crate::infra::image_manipulation::Operation::FlipHorizontally => next,
                crate::infra::image_manipulation::Operation::FlipVertically => next,
            });
        if problems.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors(problems))
        }
    }

    fn validate_source_image(
        &self,
        settings: &ValidationSettings,
        image: &DynamicImage,
    ) -> Result<(), ValidationErrors> {
        let mut problems = Vec::new();
        if image.width() > settings.max_source_image_width {
            problems.push(format!(
                "Source image width [{}] to large, must be [{}] or lower",
                image.width(),
                settings.max_source_image_width
            ));
        }
        if image.height() > settings.max_source_image_height {
            problems.push(format!(
                "Source image height [{}] to large, must be [{}] or lower",
                image.height(),
                settings.max_source_image_height
            ));
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors(problems))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_operations_validation() {
        let settings = ValidationSettings::default();
        let operations = Operations(vec![]);
        assert!(SimpleValidator
            .validate_operations(&settings, &operations)
            .is_ok());
    }

    #[test]
    fn test_non_empty_good_operations_validation() {
        let settings = ValidationSettings::default();
        let operations = Operations(vec![
            crate::infra::image_manipulation::Operation::Resize {
                width: settings.max_resize_target_width,
                height: settings.max_resize_target_height,
            },
            crate::infra::image_manipulation::Operation::FlipHorizontally,
            crate::infra::image_manipulation::Operation::FlipVertically,
        ]);
        assert!(SimpleValidator
            .validate_operations(&settings, &operations)
            .is_ok());
    }

    #[test]
    fn test_non_empty_bad_operations_validation_1() {
        let settings = ValidationSettings::default();
        let operations = Operations(vec![
            crate::infra::image_manipulation::Operation::Resize {
                width: settings.max_resize_target_width + 1,
                height: settings.max_resize_target_height,
            },
            crate::infra::image_manipulation::Operation::FlipHorizontally,
            crate::infra::image_manipulation::Operation::FlipVertically,
        ]);
        let r = SimpleValidator.validate_operations(&settings, &operations);
        let errors = r.err().unwrap();

        assert_eq!(1, errors.0.len());
        assert!(errors.0[0].starts_with("Resize target width"));
    }

    #[test]
    fn test_non_empty_bad_operations_validation_2() {
        let settings = ValidationSettings::default();
        let operations = Operations(vec![
            crate::infra::image_manipulation::Operation::Resize {
                width: settings.max_resize_target_width,
                height: settings.max_resize_target_height + 1,
            },
            crate::infra::image_manipulation::Operation::FlipHorizontally,
            crate::infra::image_manipulation::Operation::FlipVertically,
        ]);
        let r = SimpleValidator.validate_operations(&settings, &operations);
        let errors = r.err().unwrap();

        assert_eq!(1, errors.0.len());
        assert!(errors.0[0].starts_with("Resize target height"));
    }

    #[test]
    fn test_non_empty_bad_operations_validation_3() {
        let settings = ValidationSettings::default();
        let operations = Operations(vec![
            crate::infra::image_manipulation::Operation::Resize {
                width: settings.max_resize_target_width + 1,
                height: settings.max_resize_target_height + 1,
            },
            crate::infra::image_manipulation::Operation::FlipHorizontally,
            crate::infra::image_manipulation::Operation::FlipVertically,
        ]);
        let r = SimpleValidator.validate_operations(&settings, &operations);
        let errors = r.err().unwrap();

        assert_eq!(2, errors.0.len());
        assert!(errors.0[0].starts_with("Resize target width"));
        assert!(errors.0[1].starts_with("Resize target height"));
    }

    #[test]
    fn test_non_empty_good_image_validation() {
        let settings = ValidationSettings::default();
        let image = DynamicImage::new(
            settings.max_source_image_width,
            settings.max_source_image_height,
            image::ColorType::Rgb8,
        );
        assert!(SimpleValidator
            .validate_source_image(&settings, &image)
            .is_ok());
    }

    #[test]
    fn test_non_empty_bad_image_validation_1() {
        let settings = ValidationSettings::default();
        let image = DynamicImage::new(
            settings.max_source_image_width + 1,
            settings.max_source_image_height,
            image::ColorType::Rgb8,
        );
        let r = SimpleValidator.validate_source_image(&settings, &image);
        let err = r.err().unwrap();
        assert_eq!(1, err.0.len());
        assert!(err.0[0].starts_with("Source image width"));
    }

    #[test]
    fn test_non_empty_bad_image_validation_2() {
        let settings = ValidationSettings::default();
        let image = DynamicImage::new(
            settings.max_source_image_width,
            settings.max_source_image_height + 1,
            image::ColorType::Rgb8,
        );
        let r = SimpleValidator.validate_source_image(&settings, &image);
        let err = r.err().unwrap();
        assert_eq!(1, err.0.len());
        assert!(err.0[0].starts_with("Source image height"));
    }

    #[test]
    fn test_non_empty_bad_image_validation_3() {
        let settings = ValidationSettings::default();
        let image = DynamicImage::new(
            settings.max_source_image_width + 1,
            settings.max_source_image_height + 1,
            image::ColorType::Rgb8,
        );
        let r = SimpleValidator.validate_source_image(&settings, &image);
        let err = r.err().unwrap();
        assert_eq!(2, err.0.len());
        assert!(err.0[0].starts_with("Source image width"));
        assert!(err.0[1].starts_with("Source image height"));
    }
}
