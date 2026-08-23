use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

use axum::http::HeaderMap;

use crate::http::error::AppError;

// ---------------------------------------------------------------------------
// Request interval tracking (in-memory, no external dep)
// Tracks submission timestamps per IP to detect automated credential stuffing.
// ---------------------------------------------------------------------------

/// Maximum number of timestamps retained per IP. Keeps memory bounded.
const MAX_TS_PER_IP: usize = 10;

/// Window in seconds within which timestamps are considered.
const INTERVAL_WINDOW_SECS: i64 = 120;

struct IpStore {
    map: HashMap<String, VecDeque<i64>>,
    keys: VecDeque<String>,
}

static IP_INTERVALS: OnceLock<Mutex<IpStore>> = OnceLock::new();

fn ip_store() -> &'static Mutex<IpStore> {
    IP_INTERVALS.get_or_init(|| {
        Mutex::new(IpStore {
            map: HashMap::new(),
            keys: VecDeque::new(),
        })
    })
}

/// Records a request timestamp for the given IP and returns the variance
/// of recent inter-request intervals in milliseconds.
///
/// Returns `None` if there are fewer than 3 data points (not enough to score).
pub fn record_and_get_interval_variance(ip: &str) -> Option<f64> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut store = ip_store().lock().unwrap_or_else(|p| p.into_inner());

    // 10,000 maximum IPs tracked concurrently to prevent memory exhaustion (DoS).
    const MAX_TRACKED_IPS: usize = 10000;

    // Check if the IP is already tracked. If not, and we are at capacity, evict the oldest.
    if !store.map.contains_key(ip) {
        while store.map.len() >= MAX_TRACKED_IPS {
            if let Some(oldest_ip) = store.keys.pop_front() {
                store.map.remove(&oldest_ip);
            } else {
                break;
            }
        }
        store.keys.push_back(ip.to_string());
    }

    let queue = store.map.entry(ip.to_string()).or_default();

    // Evict timestamps outside the window
    let cutoff = now_ms - INTERVAL_WINDOW_SECS * 1000;
    while queue.front().is_some_and(|&ts| ts < cutoff) {
        queue.pop_front();
    }

    // Record current timestamp
    if queue.len() >= MAX_TS_PER_IP {
        queue.pop_front();
    }
    queue.push_back(now_ms);

    // Need at least 3 points to compute meaningful variance
    if queue.len() < 3 {
        return None;
    }

    // Compute inter-request intervals
    let intervals: Vec<f64> = queue
        .iter()
        .collect::<Vec<_>>()
        .windows(2)
        .map(|w| (w[1] - w[0]) as f64)
        .collect();

    let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
    let variance =
        intervals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / intervals.len() as f64;

    Some(variance)
}

// ---------------------------------------------------------------------------
// HTTP header order analysis
// ---------------------------------------------------------------------------

/// Expected header name order for a real Chrome browser (approximation).
/// Bots built on `reqwest`, `curl`, `python-httpx` have different orders.
const CHROME_HEADER_ORDER: &[&str] = &[
    "host",
    "connection",
    "content-length",
    "content-type",
    "accept",
    "origin",
    "user-agent",
    "accept-language",
    "accept-encoding",
];

/// Returns a header order anomaly score contribution (0.0 or positive).
///
/// Does NOT reject — just returns a score delta to add to the total.
pub fn score_header_order(headers: &HeaderMap) -> f64 {
    // Collect header names present in this request, in order.
    let present: Vec<String> = headers.keys().map(|k| k.as_str().to_lowercase()).collect();

    if present.is_empty() {
        return 0.0;
    }

    // Check how many CHROME_HEADER_ORDER entries are in the wrong relative position.
    let chrome_present: Vec<&str> = CHROME_HEADER_ORDER
        .iter()
        .copied()
        .filter(|h| present.contains(&h.to_string()))
        .collect();

    if chrome_present.len() < 3 {
        // Not enough headers to establish order (could be a POST with few headers).
        return 0.0;
    }

    // Find the order they appear in the actual request.
    let positions: Vec<usize> = chrome_present
        .iter()
        .filter_map(|h| present.iter().position(|p| p == h))
        .collect();

    // Count inversions — pairs where the actual order disagrees with Chrome's expected order.
    let mut inversions = 0usize;
    for i in 0..positions.len() {
        for j in (i + 1)..positions.len() {
            if positions[i] > positions[j] {
                inversions += 1;
            }
        }
    }

    let max_inversions = (chrome_present.len() * (chrome_present.len() - 1)) / 2;
    if max_inversions == 0 {
        return 0.0;
    }

    // Score: fraction of inversions × weight
    let inversion_ratio = inversions as f64 / max_inversions as f64;
    inversion_ratio * 0.12
}

// ---------------------------------------------------------------------------
// User-Agent vs Accept-Language consistency
// ---------------------------------------------------------------------------

/// Score delta for UA/Accept-Language inconsistency.
pub fn score_ua_language(headers: &HeaderMap) -> f64 {
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Only check if UA claims to be a browser.
    let is_browser_ua =
        ua.contains("Mozilla/") || ua.contains("Chrome/") || ua.contains("Firefox/");
    if !is_browser_ua {
        return 0.0; // Non-browser UA: let other signals handle it.
    }

    let accept_lang = headers
        .get("accept-language")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let mut score = 0.0;

    // Real browsers always send Accept-Language.
    if accept_lang.is_empty() {
        score += 0.10;
    }

    // Chrome always negotiates HTTP/2. If we see HTTP/1.1 with a Chrome UA,
    // it's likely a bot that spoofed the UA without matching the protocol.
    // Note: axum sees the HTTP version via the request — checked in caller.

    score
}

// ---------------------------------------------------------------------------
// Public extraction helper
// ---------------------------------------------------------------------------

pub struct HttpSignalScores {
    /// Score from request interval analysis (may be 0.0 if not enough data).
    pub interval_score: f64,
    /// Score from header order analysis.
    pub header_order_score: f64,
    /// Score from UA / Accept-Language consistency.
    pub ua_language_score: f64,
}

impl HttpSignalScores {
    pub fn total(&self) -> f64 {
        (self.interval_score + self.header_order_score + self.ua_language_score).min(1.0)
    }
}

pub fn extract_http_signals(headers: &HeaderMap, ip: &str) -> Result<HttpSignalScores, AppError> {
    let interval_score = record_and_get_interval_variance(ip)
        .map(|variance| {
            // Variance < 10ms² across 5 requests = suspiciously regular.
            if variance < 10.0 { 0.20 } else { 0.0 }
        })
        .unwrap_or(0.0);

    Ok(HttpSignalScores {
        interval_score,
        header_order_score: score_header_order(headers),
        ua_language_score: score_ua_language(headers),
    })
}
