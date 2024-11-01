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
    fn test_non_empty_vad_operations_validation() {
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
        assert!(r.is_err());
        if let Err(problems) = r {
            assert_eq!(2, problems.0.len());
        } else {
            panic!("Fail")
        }
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
    fn test_non_empty_bad_image_validation() {
        let settings = ValidationSettings::default();
        let image = DynamicImage::new(
            settings.max_source_image_width + 1,
            settings.max_source_image_height + 1,
            image::ColorType::Rgb8,
        );
        let r = SimpleValidator.validate_source_image(&settings, &image);
        assert!(r.is_err());
        if let Err(problems) = r {
            assert_eq!(2, problems.0.len());
        } else {
            panic!("Fail")
        }
    }
}
