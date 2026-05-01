use super::types::*;

pub struct ErrorClassifier;

impl ErrorClassifier {
    pub fn classify(error: &ApiError) -> ClassifiedError {
        match error {
            ApiError::RateLimit { retry_after_secs } => ClassifiedError {
                error: error.clone(),
                category: ErrorCategory::RateLimited,
                should_retry: true,
                retry_after: Some(std::time::Duration::from_secs(*retry_after_secs)),
            },
            ApiError::Timeout { .. } => ClassifiedError {
                error: error.clone(),
                category: ErrorCategory::Timeout,
                should_retry: true,
                retry_after: Some(std::time::Duration::from_secs(5)),
            },
            ApiError::NetworkError { .. } => ClassifiedError {
                error: error.clone(),
                category: ErrorCategory::Retriable,
                should_retry: true,
                retry_after: Some(std::time::Duration::from_secs(2)),
            },
            ApiError::ServerError { status_code, .. } => {
                let should_retry = *status_code >= 500 && *status_code < 600;
                ClassifiedError {
                    error: error.clone(),
                    category: ErrorCategory::ServerError,
                    should_retry,
                    retry_after: if should_retry {
                        Some(std::time::Duration::from_secs(10))
                    } else {
                        None
                    },
                }
            }
            ApiError::ContextLengthExceeded { .. } => ClassifiedError {
                error: error.clone(),
                category: ErrorCategory::NonRetriable,
                should_retry: false,
                retry_after: None,
            },
            ApiError::InvalidApiKey | ApiError::AuthenticationFailed { .. } => ClassifiedError {
                error: error.clone(),
                category: ErrorCategory::Authentication,
                should_retry: false,
                retry_after: None,
            },
            ApiError::ModelNotFound { .. } => ClassifiedError {
                error: error.clone(),
                category: ErrorCategory::NonRetriable,
                should_retry: false,
                retry_after: None,
            },
            ApiError::Unknown { .. } => ClassifiedError {
                error: error.clone(),
                category: ErrorCategory::Retriable,
                should_retry: true,
                retry_after: Some(std::time::Duration::from_secs(3)),
            },
        }
    }

    pub fn from_http_status(status_code: u16, body: &str) -> ApiError {
        match status_code {
            401 => ApiError::InvalidApiKey,
            403 => ApiError::AuthenticationFailed { message: body.to_string() },
            404 => ApiError::ModelNotFound { model: "unknown".to_string() },
            429 => {
                let retry_after = Self::parse_retry_after(body).unwrap_or(60);
                ApiError::RateLimit { retry_after_secs: retry_after }
            }
            500..=599 => ApiError::ServerError {
                status_code,
                message: body.to_string(),
            },
            _ => ApiError::Unknown { message: body.to_string() },
        }
    }

    fn parse_retry_after(body: &str) -> Option<u64> {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(retry) = json.get("retry_after") {
                return retry.as_u64().or_else(|| retry.as_str().and_then(|s| s.parse().ok()));
            }
            if let Some(error) = json.get("error") {
                if let Some(retry) = error.get("retry_after") {
                    return retry.as_u64().or_else(|| retry.as_str().and_then(|s| s.parse().ok()));
                }
            }
        }
        None
    }
}
