use prometheus::{
    Counter, Histogram, IntGauge, Registry,
    HistogramOpts, Opts, TextEncoder, Encoder,
};
use lazy_static::lazy_static;

lazy_static! {
    /// Global Prometheus registry
    pub static ref REGISTRY: Registry = Registry::new();

    // Common metrics
    pub static ref REQUESTS_TOTAL: Counter = Counter::with_opts(
        Opts::new("requests_total", "Total number of requests")
    ).unwrap();

    pub static ref LATENCY_SECONDS: Histogram = Histogram::with_opts(
        HistogramOpts::new("latency_seconds", "Request latency in seconds")
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0])
    ).unwrap();

    pub static ref ACTIVE_CONNECTIONS: IntGauge = IntGauge::with_opts(
        Opts::new("active_connections", "Number of active connections")
    ).unwrap();

    pub static ref ERRORS_TOTAL: Counter = Counter::with_opts(
        Opts::new("errors_total", "Total number of errors")
    ).unwrap();
}

/// Register all metrics with the global registry
pub fn register_metrics() {
    REGISTRY.register(Box::new(REQUESTS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(LATENCY_SECONDS.clone())).unwrap();
    REGISTRY.register(Box::new(ACTIVE_CONNECTIONS.clone())).unwrap();
    REGISTRY.register(Box::new(ERRORS_TOTAL.clone())).unwrap();
}

/// Gather metrics in Prometheus text format
pub fn gather_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registration() {
        register_metrics();
        
        REQUESTS_TOTAL.inc();
        ACTIVE_CONNECTIONS.set(10);
        LATENCY_SECONDS.observe(0.5);
        ERRORS_TOTAL.inc();

        let metrics = gather_metrics();
        assert!(metrics.contains("requests_total"));
        assert!(metrics.contains("latency_seconds"));
    }
}
