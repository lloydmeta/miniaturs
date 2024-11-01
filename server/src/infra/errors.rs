use super::validations::ValidationErrors;

#[derive(Debug)]
pub enum AppError {
    CatchAll(anyhow::Error),
    BadSignature(String),
    ValidationFailed(Vec<String>),
    UnableToDetermineFormat,
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        AppError::CatchAll(err.into())
    }
}

impl From<ValidationErrors> for AppError {
    fn from(value: ValidationErrors) -> Self {
        AppError::ValidationFailed(value.0)
    }
}
