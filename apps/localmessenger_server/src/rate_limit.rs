use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct RateLimitProfile {
    window_ms: i64,
    peer_frame_limit: u64,
    blob_request_limit: u64,
    blob_chunk_byte_limit: u64,
    health_check_limit: u64,
}

impl RateLimitProfile {
    pub fn new(
        window_ms: i64,
        peer_frame_limit: u64,
        blob_request_limit: u64,
        blob_chunk_byte_limit: u64,
        health_check_limit: u64,
    ) -> Result<Self, String> {
        if window_ms <= 0 {
            return Err("rate limit window must be greater than zero".to_string());
        }
        if peer_frame_limit == 0 {
            return Err("peer frame rate limit must be greater than zero".to_string());
        }
        if blob_request_limit == 0 {
            return Err("blob request rate limit must be greater than zero".to_string());
        }
        if blob_chunk_byte_limit == 0 {
            return Err("blob chunk byte rate limit must be greater than zero".to_string());
        }
        if health_check_limit == 0 {
            return Err("health check rate limit must be greater than zero".to_string());
        }

        Ok(Self {
            window_ms,
            peer_frame_limit,
            blob_request_limit,
            blob_chunk_byte_limit,
            health_check_limit,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RateLimitBucket {
    PeerFrame,
    BlobRequest,
    BlobChunkBytes,
    HealthCheck,
}

#[derive(Debug, Clone)]
pub struct RateLimiter {
    profile: RateLimitProfile,
    usage: Arc<Mutex<HashMap<String, DeviceUsage>>>,
}

#[derive(Debug, Default)]
struct DeviceUsage {
    peer_frames: FixedWindowCounter,
    blob_requests: FixedWindowCounter,
    blob_chunk_bytes: FixedWindowCounter,
    health_checks: FixedWindowCounter,
}

#[derive(Debug, Default)]
struct FixedWindowCounter {
    window_started_at_unix_ms: i64,
    used: u64,
}

impl RateLimiter {
    pub fn new(profile: RateLimitProfile) -> Self {
        Self {
            profile,
            usage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn allow(
        &self,
        device_id: &str,
        bucket: RateLimitBucket,
        amount: u64,
        now_unix_ms: i64,
    ) -> bool {
        let mut usage = self.usage.lock().await;
        let device = usage.entry(device_id.to_string()).or_default();

        match bucket {
            RateLimitBucket::PeerFrame => device.peer_frames.allow(
                now_unix_ms,
                self.profile.window_ms,
                self.profile.peer_frame_limit,
                amount,
            ),
            RateLimitBucket::BlobRequest => device.blob_requests.allow(
                now_unix_ms,
                self.profile.window_ms,
                self.profile.blob_request_limit,
                amount,
            ),
            RateLimitBucket::BlobChunkBytes => device.blob_chunk_bytes.allow(
                now_unix_ms,
                self.profile.window_ms,
                self.profile.blob_chunk_byte_limit,
                amount,
            ),
            RateLimitBucket::HealthCheck => device.health_checks.allow(
                now_unix_ms,
                self.profile.window_ms,
                self.profile.health_check_limit,
                amount,
            ),
        }
    }
}

impl FixedWindowCounter {
    fn allow(&mut self, now_unix_ms: i64, window_ms: i64, limit: u64, amount: u64) -> bool {
        if amount > limit {
            return false;
        }

        if self.window_started_at_unix_ms == 0
            || now_unix_ms.saturating_sub(self.window_started_at_unix_ms) >= window_ms
        {
            self.window_started_at_unix_ms = now_unix_ms;
            self.used = 0;
        }

        if self.used.saturating_add(amount) > limit {
            return false;
        }

        self.used = self.used.saturating_add(amount);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{RateLimitBucket, RateLimitProfile, RateLimiter};

    #[tokio::test]
    async fn rate_limiter_blocks_when_bucket_is_exceeded() {
        let limiter =
            RateLimiter::new(RateLimitProfile::new(1_000, 2, 1, 128, 1).expect("profile"));

        assert!(
            limiter
                .allow("alice-phone", RateLimitBucket::PeerFrame, 1, 100)
                .await
        );
        assert!(
            limiter
                .allow("alice-phone", RateLimitBucket::PeerFrame, 1, 200)
                .await
        );
        assert!(
            !limiter
                .allow("alice-phone", RateLimitBucket::PeerFrame, 1, 300)
                .await
        );
    }

    #[tokio::test]
    async fn rate_limiter_tracks_devices_and_buckets_independently() {
        let limiter = RateLimiter::new(RateLimitProfile::new(1_000, 1, 1, 64, 1).expect("profile"));

        assert!(
            limiter
                .allow("alice-phone", RateLimitBucket::BlobRequest, 1, 100)
                .await
        );
        assert!(
            limiter
                .allow("bob-phone", RateLimitBucket::BlobRequest, 1, 100)
                .await
        );
        assert!(
            limiter
                .allow("alice-phone", RateLimitBucket::BlobChunkBytes, 32, 100)
                .await
        );
        assert!(
            limiter
                .allow("alice-phone", RateLimitBucket::BlobChunkBytes, 32, 150)
                .await
        );
        assert!(
            !limiter
                .allow("alice-phone", RateLimitBucket::BlobChunkBytes, 1, 200)
                .await
        );
    }

    #[tokio::test]
    async fn rate_limiter_resets_after_window_rollover() {
        let limiter = RateLimiter::new(RateLimitProfile::new(500, 1, 1, 32, 1).expect("profile"));

        assert!(
            limiter
                .allow("alice-phone", RateLimitBucket::HealthCheck, 1, 100)
                .await
        );
        assert!(
            !limiter
                .allow("alice-phone", RateLimitBucket::HealthCheck, 1, 200)
                .await
        );
        assert!(
            limiter
                .allow("alice-phone", RateLimitBucket::HealthCheck, 1, 700)
                .await
        );
    }
}
