use tracing_subscriber::EnvFilter;

/// Initialize structured logging with JSON format
pub fn init() {
    init_with_level("info")
}

/// Initialize logging with specific level
pub fn init_with_level(level: &str) {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(level))
        )
        .json()
        .init();
}

/// Initialize logging for tests (plain format)
pub fn init_test() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("debug"))
        .with_test_writer()
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::{info, warn, error};

    #[test]
    fn test_logging_init() {
        init_test();
        info!("Test info message");
        warn!("Test warning message");
        error!("Test error message");
    }
}
