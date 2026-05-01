use anyhow::Result;
use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

pub struct LogManager {
    _guard: Option<WorkerGuard>,
}

impl LogManager {
    pub fn init(log_dir: &str, verbose: bool) -> Result<Self> {
        let log_dir = PathBuf::from(log_dir);
        std::fs::create_dir_all(&log_dir)?;

        let file_appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("aio-agent")
            .filename_suffix("log")
            .build(&log_dir)?;

        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .with_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                EnvFilter::new("info")
            }));

        let console_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_level(true)
            .with_ansi(true)
            .with_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                if verbose {
                    EnvFilter::new("debug")
                } else {
                    EnvFilter::new("info")
                }
            }));

        tracing_subscriber::registry()
            .with(file_layer)
            .with(console_layer)
            .init();

        Ok(Self {
            _guard: Some(guard),
        })
    }

    pub fn init_simple() -> Result<Self> {
        let home = dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".aio-agent")
            .join("logs");

        Self::init(home.to_str().unwrap(), false)
    }
}
