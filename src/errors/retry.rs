use super::types::{ApiError, ClassifiedError, ErrorCategory};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum RetryResult {
    Retry { delay: Duration },
    Abort,
    Success,
}

pub struct RetryPolicy {
    pub max_retries: usize,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter: bool,
}

impl RetryPolicy {
    pub fn new(max_retries: usize, base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            max_retries,
            base_delay,
            max_delay,
            jitter: true,
        }
    }

    pub fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter: true,
        }
    }

    pub fn evaluate(&self, attempt: usize, classified: &ClassifiedError) -> RetryResult {
        if !classified.should_retry {
            return RetryResult::Abort;
        }
        if attempt >= self.max_retries {
            return RetryResult::Abort;
        }

        let base = classified.retry_after.unwrap_or(self.base_delay);
        let exponential = base * 2u32.pow(attempt as u32);
        let delay = exponential.min(self.max_delay);

        if self.jitter {
            let jitter_factor = fastrand::f64() * 0.3 + 0.85;
            RetryResult::Retry {
                delay: Duration::from_millis((delay.as_millis() as f64 * jitter_factor) as u64),
            }
        } else {
            RetryResult::Retry { delay }
        }
    }

    pub async fn execute_with_retry<F, Fut, T>(
        &self,
        mut operation: F,
    ) -> Result<T, ApiError>
    where
        F: FnMut(usize) -> Fut,
        Fut: std::future::Future<Output = Result<T, ApiError>>,
    {
        let mut attempt = 0;
        loop {
            match operation(attempt).await {
                Ok(value) => return Ok(value),
                Err(error) => {
                    let classified = crate::errors::ErrorClassifier::classify(&error);
                    match self.evaluate(attempt, &classified) {
                        RetryResult::Retry { delay } => {
                            tokio::time::sleep(delay).await;
                            attempt += 1;
                        }
                        RetryResult::Abort => return Err(error),
                        RetryResult::Success => unreachable!(),
                    }
                }
            }
        }
    }
}
