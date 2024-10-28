#[derive(Debug)]
pub enum AppError {
    CatchAll(anyhow::Error),
    BadSignature(String),
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
