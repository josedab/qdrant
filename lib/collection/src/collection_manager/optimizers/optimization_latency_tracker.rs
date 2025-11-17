/// Latency tracking for segment optimization operations
///
/// This module implements RFC-0003: Async Segment Optimization
/// Goal: Ensure P99 search latency < 50ms during optimization (down from 500ms+)
///
/// The tracker monitors search operation latency, specifically during optimization
/// periods, to verify that the async optimization implementation meets the RFC goals.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::Serialize;

/// Latency bucket boundaries in microseconds for histogram tracking
/// Buckets: [0-1ms, 1-5ms, 5-10ms, 10-25ms, 25-50ms, 50-100ms, 100-250ms, 250-500ms, 500ms+]
const LATENCY_BUCKETS_US: &[u64] = &[
    1_000,    // 1ms
    5_000,    // 5ms
    10_000,   // 10ms
    25_000,   // 25ms
    50_000,   // 50ms - RFC target for P99
    100_000,  // 100ms
    250_000,  // 250ms
    500_000,  // 500ms - current P99 baseline
    1_000_000, // 1s
];

/// Latency histogram for tracking search operation latencies
#[derive(Debug, Clone)]
pub struct LatencyHistogram {
    /// Histogram buckets counting operations in each latency range
    buckets: Arc<[AtomicUsize; LATENCY_BUCKETS_US.len() + 1]>,
    /// Total operation count
    total_count: Arc<AtomicUsize>,
    /// Sum of all latencies in microseconds for calculating mean
    total_latency_us: Arc<AtomicU64>,
    /// Minimum latency observed in microseconds
    min_latency_us: Arc<AtomicU64>,
    /// Maximum latency observed in microseconds
    max_latency_us: Arc<AtomicU64>,
}

impl Default for LatencyHistogram {
    fn default() -> Self {
        Self {
            buckets: Arc::new([
                AtomicUsize::new(0),
                AtomicUsize::new(0),
                AtomicUsize::new(0),
                AtomicUsize::new(0),
                AtomicUsize::new(0),
                AtomicUsize::new(0),
                AtomicUsize::new(0),
                AtomicUsize::new(0),
                AtomicUsize::new(0),
                AtomicUsize::new(0),
            ]),
            total_count: Arc::new(AtomicUsize::new(0)),
            total_latency_us: Arc::new(AtomicU64::new(0)),
            min_latency_us: Arc::new(AtomicU64::new(u64::MAX)),
            max_latency_us: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl LatencyHistogram {
    /// Record a latency measurement
    #[inline]
    pub fn record(&self, duration: Duration) {
        let latency_us = duration.as_micros() as u64;

        // Update histogram bucket
        let bucket_idx = LATENCY_BUCKETS_US
            .iter()
            .position(|&threshold| latency_us < threshold)
            .unwrap_or(LATENCY_BUCKETS_US.len());
        self.buckets[bucket_idx].fetch_add(1, Ordering::Relaxed);

        // Update statistics
        self.total_count.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us.fetch_add(latency_us, Ordering::Relaxed);

        // Update min (using fetch_min when stable, for now use compare_exchange loop)
        let mut current_min = self.min_latency_us.load(Ordering::Relaxed);
        while latency_us < current_min {
            match self.min_latency_us.compare_exchange_weak(
                current_min,
                latency_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }

        // Update max (using fetch_max when stable, for now use compare_exchange loop)
        let mut current_max = self.max_latency_us.load(Ordering::Relaxed);
        while latency_us > current_max {
            match self.max_latency_us.compare_exchange_weak(
                current_max,
                latency_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }
    }

    /// Get snapshot of current statistics
    pub fn snapshot(&self) -> LatencyStats {
        let total_count = self.total_count.load(Ordering::Relaxed);
        let total_latency_us = self.total_latency_us.load(Ordering::Relaxed);
        let min_latency_us = self.min_latency_us.load(Ordering::Relaxed);
        let max_latency_us = self.max_latency_us.load(Ordering::Relaxed);

        let buckets: Vec<usize> = self
            .buckets
            .iter()
            .map(|b| b.load(Ordering::Relaxed))
            .collect();

        let mean_latency_us = if total_count > 0 {
            total_latency_us / total_count as u64
        } else {
            0
        };

        // Calculate P99 from histogram
        let p99_threshold = (total_count as f64 * 0.99) as usize;
        let mut accumulated = 0;
        let mut p99_latency_us = 0;
        for (i, &count) in buckets.iter().enumerate() {
            accumulated += count;
            if accumulated >= p99_threshold {
                p99_latency_us = if i < LATENCY_BUCKETS_US.len() {
                    LATENCY_BUCKETS_US[i]
                } else {
                    max_latency_us
                };
                break;
            }
        }

        LatencyStats {
            count: total_count,
            mean_latency_us,
            min_latency_us: if min_latency_us == u64::MAX {
                0
            } else {
                min_latency_us
            },
            max_latency_us,
            p99_latency_us,
            histogram: buckets,
        }
    }

    /// Reset all statistics
    pub fn reset(&self) {
        for bucket in self.buckets.iter() {
            bucket.store(0, Ordering::Relaxed);
        }
        self.total_count.store(0, Ordering::Relaxed);
        self.total_latency_us.store(0, Ordering::Relaxed);
        self.min_latency_us.store(u64::MAX, Ordering::Relaxed);
        self.max_latency_us.store(0, Ordering::Relaxed);
    }
}

/// Snapshot of latency statistics
#[derive(Debug, Clone, Serialize)]
pub struct LatencyStats {
    /// Total number of operations measured
    pub count: usize,
    /// Mean latency in microseconds
    pub mean_latency_us: u64,
    /// Minimum latency in microseconds
    pub min_latency_us: u64,
    /// Maximum latency in microseconds
    pub max_latency_us: u64,
    /// P99 latency in microseconds (RFC target: < 50,000 us = 50ms)
    pub p99_latency_us: u64,
    /// Histogram bucket counts
    pub histogram: Vec<usize>,
}

impl LatencyStats {
    /// Check if P99 latency meets the RFC-0003 goal of < 50ms
    pub fn meets_rfc_goal(&self) -> bool {
        self.p99_latency_us < 50_000 // 50ms in microseconds
    }

    /// Get P99 latency in milliseconds
    pub fn p99_latency_ms(&self) -> f64 {
        self.p99_latency_us as f64 / 1000.0
    }

    /// Get mean latency in milliseconds
    pub fn mean_latency_ms(&self) -> f64 {
        self.mean_latency_us as f64 / 1000.0
    }
}

/// Global latency tracker for optimization operations
#[derive(Clone)]
pub struct OptimizationLatencyTracker {
    /// Latency tracking for search operations during optimization
    search_during_optimization: Arc<RwLock<LatencyHistogram>>,
    /// Latency tracking for search operations during normal (non-optimization) periods
    search_normal: Arc<RwLock<LatencyHistogram>>,
    /// Whether optimization is currently active
    optimization_active: Arc<AtomicUsize>,
}

impl Default for OptimizationLatencyTracker {
    fn default() -> Self {
        Self {
            search_during_optimization: Arc::new(RwLock::new(LatencyHistogram::default())),
            search_normal: Arc::new(RwLock::new(LatencyHistogram::default())),
            optimization_active: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl OptimizationLatencyTracker {
    /// Start tracking an optimization operation
    pub fn start_optimization(&self) {
        self.optimization_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Stop tracking an optimization operation
    pub fn end_optimization(&self) {
        self.optimization_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Check if optimization is currently active
    #[inline]
    pub fn is_optimizing(&self) -> bool {
        self.optimization_active.load(Ordering::Relaxed) > 0
    }

    /// Record a search operation latency
    #[inline]
    pub fn record_search(&self, duration: Duration) {
        if self.is_optimizing() {
            self.search_during_optimization.read().record(duration);
        } else {
            self.search_normal.read().record(duration);
        }
    }

    /// Get snapshot of search latency during optimization
    pub fn get_optimization_stats(&self) -> LatencyStats {
        self.search_during_optimization.read().snapshot()
    }

    /// Get snapshot of search latency during normal operation
    pub fn get_normal_stats(&self) -> LatencyStats {
        self.search_normal.read().snapshot()
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.search_during_optimization.read().reset();
        self.search_normal.read().reset();
    }
}

/// RAII guard for tracking operation latency
pub struct LatencyMeasurement {
    start: Instant,
    tracker: OptimizationLatencyTracker,
}

impl LatencyMeasurement {
    /// Start measuring latency for a search operation
    #[inline]
    pub fn start(tracker: OptimizationLatencyTracker) -> Self {
        Self {
            start: Instant::now(),
            tracker,
        }
    }
}

impl Drop for LatencyMeasurement {
    #[inline]
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        self.tracker.record_search(duration);
    }
}

/// RAII guard for marking an optimization period
pub struct OptimizationPeriod {
    tracker: OptimizationLatencyTracker,
}

impl OptimizationPeriod {
    /// Start an optimization period
    pub fn start(tracker: OptimizationLatencyTracker) -> Self {
        tracker.start_optimization();
        Self { tracker }
    }
}

impl Drop for OptimizationPeriod {
    fn drop(&mut self) {
        self.tracker.end_optimization();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_latency_histogram_basic() {
        let histogram = LatencyHistogram::default();

        // Record some latencies
        histogram.record(Duration::from_micros(500)); // < 1ms
        histogram.record(Duration::from_micros(3000)); // 1-5ms
        histogram.record(Duration::from_micros(30000)); // 25-50ms
        histogram.record(Duration::from_micros(60000)); // 50-100ms

        let stats = histogram.snapshot();
        assert_eq!(stats.count, 4);
        assert!(stats.min_latency_us <= 500);
        assert!(stats.max_latency_us >= 60000);
    }

    #[test]
    fn test_p99_calculation() {
        let histogram = LatencyHistogram::default();

        // Record 100 operations: 99 fast, 1 slow
        for _ in 0..99 {
            histogram.record(Duration::from_micros(1000)); // 1ms
        }
        histogram.record(Duration::from_micros(100000)); // 100ms

        let stats = histogram.snapshot();
        assert_eq!(stats.count, 100);
        // P99 should be in the fast range
        assert!(stats.p99_latency_us < 100000);
    }

    #[test]
    fn test_rfc_goal_check() {
        let histogram = LatencyHistogram::default();

        // All operations under 50ms
        for _ in 0..100 {
            histogram.record(Duration::from_micros(10000)); // 10ms
        }

        let stats = histogram.snapshot();
        assert!(stats.meets_rfc_goal(), "Should meet RFC goal of P99 < 50ms");
        assert!(stats.p99_latency_ms() < 50.0);
    }

    #[test]
    fn test_optimization_tracker() {
        let tracker = OptimizationLatencyTracker::default();

        assert!(!tracker.is_optimizing());

        // Start optimization
        tracker.start_optimization();
        assert!(tracker.is_optimizing());

        // Record some searches during optimization
        tracker.record_search(Duration::from_micros(10000));
        tracker.record_search(Duration::from_micros(20000));

        let opt_stats = tracker.get_optimization_stats();
        assert_eq!(opt_stats.count, 2);

        let normal_stats = tracker.get_normal_stats();
        assert_eq!(normal_stats.count, 0);

        // End optimization
        tracker.end_optimization();
        assert!(!tracker.is_optimizing());

        // Record searches during normal operation
        tracker.record_search(Duration::from_micros(5000));

        let normal_stats = tracker.get_normal_stats();
        assert_eq!(normal_stats.count, 1);
    }

    #[test]
    fn test_measurement_guard() {
        let tracker = OptimizationLatencyTracker::default();
        tracker.start_optimization();

        {
            let _measurement = LatencyMeasurement::start(tracker.clone());
            thread::sleep(Duration::from_millis(1));
        } // measurement dropped here, latency recorded

        let stats = tracker.get_optimization_stats();
        assert_eq!(stats.count, 1);
        assert!(stats.min_latency_us >= 1000); // at least 1ms
    }

    #[test]
    fn test_optimization_period_guard() {
        let tracker = OptimizationLatencyTracker::default();
        assert!(!tracker.is_optimizing());

        {
            let _period = OptimizationPeriod::start(tracker.clone());
            assert!(tracker.is_optimizing());
        } // period dropped here

        assert!(!tracker.is_optimizing());
    }
}
