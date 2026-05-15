use rmcp::ErrorData;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("config error: {0}")]
    Config(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("YouTrack {status}: {message}")]
    Api { status: u16, message: String },
    #[error("{0}")]
    Bad(String),
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Network(e.to_string())
    }
}

impl From<AppError> for ErrorData {
    fn from(e: AppError) -> Self {
        match e {
            AppError::Bad(m) => ErrorData::invalid_params(m, None),
            AppError::Api { status: 404, message } => {
                ErrorData::resource_not_found(format!("YouTrack 404: {message}"), None)
            }
            other => ErrorData::internal_error(other.to_string(), None),
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
