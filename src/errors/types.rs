use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum ApiError {
    #[error("Rate limit exceeded")]
    RateLimit { retry_after_secs: u64 },

    #[error("Context length exceeded: {message}")]
    ContextLengthExceeded { message: String },

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Model not found: {model}")]
    ModelNotFound { model: String },

    #[error("Timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Network error: {message}")]
    NetworkError { message: String },

    #[error("Server error: {status_code} - {message}")]
    ServerError { status_code: u16, message: String },

    #[error("Authentication failed: {message}")]
    AuthenticationFailed { message: String },

    #[error("Unknown error: {message}")]
    Unknown { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCategory {
    Retriable,
    NonRetriable,
    RateLimited,
    Authentication,
    Timeout,
    ServerError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedError {
    pub error: ApiError,
    pub category: ErrorCategory,
    pub should_retry: bool,
    pub retry_after: Option<std::time::Duration>,
}
