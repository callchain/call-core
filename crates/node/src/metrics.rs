//! Monitoring & Metrics
//!
//! Provides Prometheus-compatible metrics for monitoring the node.
//! Includes consensus, network, and transaction metrics.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Metric types supported by the system
#[derive(Debug, Clone)]
pub enum MetricValue {
    Counter(u64),
    Gauge(i64),
    Histogram(Vec<f64>),
}

/// A single metric
#[derive(Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub help: String,
    pub value: MetricValue,
    pub labels: HashMap<String, String>,
}

/// Metrics registry
pub struct MetricsRegistry {
    counters: HashMap<String, Arc<AtomicU64>>,
    gauges: HashMap<String, Arc<AtomicU64>>,
    histograms: HashMap<String, Vec<f64>>,
    start_time: Instant,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
            start_time: Instant::now(),
        }
    }

    /// Register a counter
    pub fn register_counter(&mut self, name: &str, _help: &str) -> Arc<AtomicU64> {
        let counter = Arc::new(AtomicU64::new(0));
        self.counters.insert(name.to_string(), counter.clone());
        counter
    }

    /// Increment a counter
    pub fn increment(&self, name: &str) {
        if let Some(counter) = self.counters.get(name) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Add to a counter
    pub fn add(&self, name: &str, value: u64) {
        if let Some(counter) = self.counters.get(name) {
            counter.fetch_add(value, Ordering::Relaxed);
        }
    }

    /// Set a gauge
    pub fn set_gauge(&mut self, name: &str, value: u64) {
        let gauge = self
            .gauges
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)));
        gauge.store(value, Ordering::Relaxed);
    }

    /// Record a histogram value
    pub fn record_histogram(&mut self, name: &str, value: f64) {
        self.histograms
            .entry(name.to_string())
            .or_default()
            .push(value);
    }

    /// Get counter value
    pub fn get_counter(&self, name: &str) -> u64 {
        self.counters
            .get(name)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get gauge value
    pub fn get_gauge(&self, name: &str) -> i64 {
        self.gauges
            .get(name)
            .map(|g| g.load(Ordering::Relaxed) as i64)
            .unwrap_or(0)
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        // Export counters
        for (name, counter) in &self.counters {
            let value = counter.load(Ordering::Relaxed);
            output.push_str(&format!("# TYPE {} counter\n", name));
            output.push_str(&format!("{} {} {}\n", name, value, timestamp));
        }

        // Export gauges
        for (name, gauge) in &self.gauges {
            let value = gauge.load(Ordering::Relaxed);
            output.push_str(&format!("# TYPE {} gauge\n", name));
            output.push_str(&format!("{} {} {}\n", name, value, timestamp));
        }

        // Export histograms
        for (name, values) in &self.histograms {
            if !values.is_empty() {
                output.push_str(&format!("# TYPE {} histogram\n", name));
                let sum: f64 = values.iter().sum();
                let count = values.len();
                output.push_str(&format!("{}_sum {}\n", name, sum));
                output.push_str(&format!("{}_count {}\n", name, count));

                // Calculate percentiles
                let mut sorted = values.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                for percentile in [50.0, 90.0, 99.0] {
                    let idx = ((percentile / 100.0) * sorted.len() as f64) as usize;
                    let idx = idx.min(sorted.len() - 1);
                    output.push_str(&format!(
                        "{}_bucket{{le=\"{}\"}} {}\n",
                        name, percentile, sorted[idx]
                    ));
                }
            }
        }

        // Add uptime metric
        output.push_str("# TYPE node_uptime_seconds gauge\n");
        output.push_str(&format!("node_uptime_seconds {} {}\n", self.uptime_seconds(), timestamp));

        output
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Standard metric names
pub mod metrics {
    // Consensus metrics
    pub const CONSENSUS_ROUNDS: &str = "consensus_rounds_total";
    pub const CONSENSUS_DURATION: &str = "consensus_duration_seconds";
    pub const LEDGER_CLOSE_TIME: &str = "ledger_close_time_seconds";
    pub const LEDGER_TRANSACTIONS: &str = "ledger_transactions_total";
    pub const VALIDATION_RECEIVED: &str = "consensus_validations_received_total";
    pub const PROPOSAL_RECEIVED: &str = "consensus_proposals_received_total";

    // Network metrics
    pub const PEERS_CONNECTED: &str = "network_peers_connected";
    pub const PEERS_DISCOVERED: &str = "network_peers_discovered_total";
    pub const MESSAGES_SENT: &str = "network_messages_sent_total";
    pub const MESSAGES_RECEIVED: &str = "network_messages_received_total";
    pub const BYTES_SENT: &str = "network_bytes_sent_total";
    pub const BYTES_RECEIVED: &str = "network_bytes_received_total";

    // Transaction metrics
    pub const TRANSACTIONS_SUBMITTED: &str = "transactions_submitted_total";
    pub const TRANSACTIONS_PROCESSED: &str = "transactions_processed_total";
    pub const TRANSACTIONS_ACCEPTED: &str = "transactions_accepted_total";
    pub const TRANSACTIONS_REJECTED: &str = "transactions_rejected_total";
    pub const TRANSACTION_QUEUE_SIZE: &str = "transaction_queue_size";
    pub const TRANSACTION_PROCESSING_TIME: &str = "transaction_processing_duration_seconds";

    // RPC metrics
    pub const RPC_REQUESTS: &str = "rpc_requests_total";
    pub const RPC_ERRORS: &str = "rpc_errors_total";
    pub const RPC_DURATION: &str = "rpc_request_duration_seconds";

    // Storage metrics
    pub const STORAGE_READS: &str = "storage_reads_total";
    pub const STORAGE_WRITES: &str = "storage_writes_total";
    pub const STORAGE_READ_TIME: &str = "storage_read_duration_seconds";
    pub const STORAGE_WRITE_TIME: &str = "storage_write_duration_seconds";
}

/// Metrics exporter for HTTP endpoint
pub struct MetricsExporter {
    registry: Arc<std::sync::Mutex<MetricsRegistry>>,
}

impl MetricsExporter {
    pub fn new(registry: Arc<std::sync::Mutex<MetricsRegistry>>) -> Self {
        Self { registry }
    }

    /// Handle metrics request (returns Prometheus formatted metrics)
    pub fn handle_request(&self) -> String {
        let registry = self.registry.lock().unwrap();
        registry.export_prometheus()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let mut registry = MetricsRegistry::new();
        registry.register_counter("test_counter", "A test counter");

        registry.increment("test_counter");
        registry.increment("test_counter");
        registry.add("test_counter", 5);

        assert_eq!(registry.get_counter("test_counter"), 7);
    }

    #[test]
    fn test_gauge() {
        let mut registry = MetricsRegistry::new();
        registry.set_gauge("test_gauge", 100);
        assert_eq!(registry.get_gauge("test_gauge"), 100);

        registry.set_gauge("test_gauge", 50);
        assert_eq!(registry.get_gauge("test_gauge"), 50);
    }

    #[test]
    fn test_histogram() {
        let mut registry = MetricsRegistry::new();
        registry.record_histogram("test_histogram", 1.0);
        registry.record_histogram("test_histogram", 2.0);
        registry.record_histogram("test_histogram", 3.0);

        let export = registry.export_prometheus();
        assert!(export.contains("test_histogram_sum 6"));
        assert!(export.contains("test_histogram_count 3"));
    }

    #[test]
    fn test_prometheus_export() {
        let mut registry = MetricsRegistry::new();
        registry.register_counter("test_counter", "Test");
        registry.increment("test_counter");

        let export = registry.export_prometheus();
        assert!(export.contains("# TYPE test_counter counter"));
        assert!(export.contains("test_counter 1"));
        assert!(export.contains("node_uptime_seconds"));
    }

    #[test]
    fn test_uptime() {
        let registry = MetricsRegistry::new();
        std::thread::sleep(Duration::from_millis(10));
        assert!(registry.uptime_seconds() >= 0);
    }
}
