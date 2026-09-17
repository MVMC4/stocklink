//! Minimal Prometheus-style process metrics.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

/// Upper bound (seconds) of each latency bucket, matching Prometheus' own
/// default histogram buckets — fine-grained near typical request latency,
/// coarse at the tail.
const LATENCY_BUCKETS_SECS: [f64; 11] = [
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

#[derive(Debug, Default)]
struct RouteHistogram {
    /// Cumulative count per bucket (Prometheus histograms are cumulative:
    /// bucket `i` counts every observation `<= LATENCY_BUCKETS_SECS[i]`).
    bucket_counts: [AtomicU64; LATENCY_BUCKETS_SECS.len()],
    count: AtomicU64,
    sum_micros: AtomicU64,
}

impl RouteHistogram {
    fn observe(&self, duration: std::time::Duration) {
        let secs = duration.as_secs_f64();
        for (i, upper) in LATENCY_BUCKETS_SECS.iter().enumerate() {
            if secs <= *upper {
                self.bucket_counts[i].fetch_add(1, Ordering::Relaxed);
            }
        }
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum_micros
            .fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }
}

#[derive(Debug, Default)]
pub struct Metrics {
    requests_total: AtomicU64,
    requests_in_flight: AtomicU64,
    responses_2xx: AtomicU64,
    responses_4xx: AtomicU64,
    responses_5xx: AtomicU64,
    rate_limited_total: AtomicU64,
    /// Keyed by `(method, route_pattern)` — the route's matched path pattern
    /// (e.g. `/v1/orders/:id`), never the raw URL, so distinct ids don't
    /// explode cardinality.
    route_latency: RwLock<HashMap<(String, String), RouteHistogram>>,
}

impl Metrics {
    pub fn request_started(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        self.requests_in_flight.fetch_add(1, Ordering::Relaxed);
    }

    pub fn request_finished(&self, status: u16) {
        self.requests_in_flight.fetch_sub(1, Ordering::Relaxed);
        match status {
            200..=299 => {
                self.responses_2xx.fetch_add(1, Ordering::Relaxed);
            }
            400..=499 => {
                self.responses_4xx.fetch_add(1, Ordering::Relaxed);
            }
            500..=599 => {
                self.responses_5xx.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    pub fn rate_limited(&self) {
        self.rate_limited_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record one request's latency against its matched route pattern.
    pub fn observe_route_latency(&self, method: &str, route: &str, duration: std::time::Duration) {
        // Fast path: the route already has a histogram — only a read lock.
        if let Some(hist) = self
            .route_latency
            .read()
            .expect("route_latency lock poisoned")
            .get(&(method.to_string(), route.to_string()))
        {
            hist.observe(duration);
            return;
        }
        // Slow path: first observation for this route — take the write lock
        // and insert, re-checking in case another request raced us here.
        let mut map = self
            .route_latency
            .write()
            .expect("route_latency lock poisoned");
        map.entry((method.to_string(), route.to_string()))
            .or_default()
            .observe(duration);
    }

    fn render_route_latency(&self) -> String {
        let map = self
            .route_latency
            .read()
            .expect("route_latency lock poisoned");
        if map.is_empty() {
            return String::new();
        }
        let mut out = String::from(
            "# HELP stocklink_http_request_duration_seconds Request latency by route.\n\
             # TYPE stocklink_http_request_duration_seconds histogram\n",
        );
        for ((method, route), hist) in map.iter() {
            for (i, upper) in LATENCY_BUCKETS_SECS.iter().enumerate() {
                let cumulative = hist.bucket_counts[i].load(Ordering::Relaxed);
                out.push_str(&format!(
                    "stocklink_http_request_duration_seconds_bucket{{method=\"{method}\",route=\"{route}\",le=\"{upper}\"}} {cumulative}\n"
                ));
            }
            let count = hist.count.load(Ordering::Relaxed);
            out.push_str(&format!(
                "stocklink_http_request_duration_seconds_bucket{{method=\"{method}\",route=\"{route}\",le=\"+Inf\"}} {count}\n"
            ));
            let sum_secs = hist.sum_micros.load(Ordering::Relaxed) as f64 / 1_000_000.0;
            out.push_str(&format!(
                "stocklink_http_request_duration_seconds_sum{{method=\"{method}\",route=\"{route}\"}} {sum_secs}\n"
            ));
            out.push_str(&format!(
                "stocklink_http_request_duration_seconds_count{{method=\"{method}\",route=\"{route}\"}} {count}\n"
            ));
        }
        out
    }

    /// Health/build gauges appended to the `/metrics` output so a single
    /// Prometheus scrape drives both the API and the health dashboard without a
    /// separate exporter. `db_up`/`redis_up` come from a cheap readiness ping
    /// on each scrape (`HealthService::readiness`).
    pub fn render_health(db_up: bool, redis_up: bool, version: &str) -> String {
        let g = |up: bool| if up { 1 } else { 0 };
        format!(
            concat!(
                "# HELP stocklink_up Whether the API process is serving (always 1 when scraped).\n",
                "# TYPE stocklink_up gauge\n",
                "stocklink_up 1\n",
                "# HELP stocklink_dependency_up Whether a downstream dependency is reachable.\n",
                "# TYPE stocklink_dependency_up gauge\n",
                "stocklink_dependency_up{{dependency=\"database\"}} {}\n",
                "stocklink_dependency_up{{dependency=\"redis\"}} {}\n",
                "# HELP stocklink_build_info Build metadata (value is always 1).\n",
                "# TYPE stocklink_build_info gauge\n",
                "stocklink_build_info{{version=\"{}\"}} 1\n",
            ),
            g(db_up),
            g(redis_up),
            version,
        )
    }

    pub fn render_prometheus(&self) -> String {
        let counters = format!(
            concat!(
                "# HELP stocklink_http_requests_total Total HTTP requests.\n",
                "# TYPE stocklink_http_requests_total counter\n",
                "stocklink_http_requests_total {}\n",
                "# HELP stocklink_http_requests_in_flight In-flight HTTP requests.\n",
                "# TYPE stocklink_http_requests_in_flight gauge\n",
                "stocklink_http_requests_in_flight {}\n",
                "# HELP stocklink_http_responses_total HTTP responses by status class.\n",
                "# TYPE stocklink_http_responses_total counter\n",
                "stocklink_http_responses_total{{class=\"2xx\"}} {}\n",
                "stocklink_http_responses_total{{class=\"4xx\"}} {}\n",
                "stocklink_http_responses_total{{class=\"5xx\"}} {}\n",
                "# HELP stocklink_rate_limited_requests_total Requests rejected by the rate limiter.\n",
                "# TYPE stocklink_rate_limited_requests_total counter\n",
                "stocklink_rate_limited_requests_total {}\n",
            ),
            self.requests_total.load(Ordering::Relaxed),
            self.requests_in_flight.load(Ordering::Relaxed),
            self.responses_2xx.load(Ordering::Relaxed),
            self.responses_4xx.load(Ordering::Relaxed),
            self.responses_5xx.load(Ordering::Relaxed),
            self.rate_limited_total.load(Ordering::Relaxed),
        );
        format!("{counters}{}", self.render_route_latency())
    }
}

#[cfg(test)]
mod tests {
    use super::Metrics;

    #[test]
    fn renders_prometheus_counters() {
        let metrics = Metrics::default();
        metrics.request_started();
        metrics.request_finished(200);
        metrics.rate_limited();

        let body = metrics.render_prometheus();
        assert!(body.contains("stocklink_http_requests_total 1"));
        assert!(body.contains("stocklink_http_responses_total{class=\"2xx\"} 1"));
        assert!(body.contains("stocklink_rate_limited_requests_total 1"));
    }

    #[test]
    fn renders_route_latency_histogram_with_cumulative_buckets() {
        let metrics = Metrics::default();
        metrics.observe_route_latency("GET", "/v1/orders/:id", std::time::Duration::from_millis(3));
        metrics.observe_route_latency(
            "GET",
            "/v1/orders/:id",
            std::time::Duration::from_millis(300),
        );

        let body = metrics.render_prometheus();
        assert!(body.contains(
            "stocklink_http_request_duration_seconds_bucket{method=\"GET\",route=\"/v1/orders/:id\",le=\"0.005\"} 1"
        ));
        // The 300ms observation falls into every bucket >= 0.5s, cumulatively.
        assert!(body.contains(
            "stocklink_http_request_duration_seconds_bucket{method=\"GET\",route=\"/v1/orders/:id\",le=\"0.5\"} 2"
        ));
        assert!(body.contains(
            "stocklink_http_request_duration_seconds_bucket{method=\"GET\",route=\"/v1/orders/:id\",le=\"+Inf\"} 2"
        ));
        assert!(body.contains(
            "stocklink_http_request_duration_seconds_count{method=\"GET\",route=\"/v1/orders/:id\"} 2"
        ));
    }

    #[test]
    fn renders_health_and_build_gauges() {
        let body = Metrics::render_health(true, false, "0.1.0");
        assert!(body.contains("stocklink_up 1"));
        assert!(body.contains("stocklink_dependency_up{dependency=\"database\"} 1"));
        assert!(body.contains("stocklink_dependency_up{dependency=\"redis\"} 0"));
        assert!(body.contains("stocklink_build_info{version=\"0.1.0\"} 1"));
    }
}
