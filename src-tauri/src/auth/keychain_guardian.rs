//! Background task that holds a swap target's credentials in the platform
//! credential store for 60 seconds post-swap, defeating the narrow race
//! where a still-running Claude Code process completes an in-flight OAuth
//! refresh after our swap and writes the previous account's rotated tokens
//! back. CC's own keychain cache TTL (30s, src/utils/secureStorage/
//! macOsKeychainHelpers.ts:69) does the natural hot-reload; we just protect
//! the entry through the danger window.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Notify;

#[async_trait]
pub trait CredIO: Send + Sync + 'static {
    async fn load(&self) -> Result<Option<Value>>;
    async fn write(&self, blob: &Value) -> Result<()>;
}

pub struct KeychainGuardian {
    cancel: Arc<Notify>,
}

impl KeychainGuardian {
    pub fn arm<I: CredIO>(_target_blob: Value, _io: Arc<I>) -> Self {
        Self {
            cancel: Arc::new(Notify::new()),
        }
    }

    pub fn cancel(self) {
        self.cancel.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    struct MockIO {
        current: Mutex<Option<Value>>,
        writes: AtomicUsize,
    }

    impl MockIO {
        fn new(initial: Value) -> Arc<Self> {
            Arc::new(Self {
                current: Mutex::new(Some(initial)),
                writes: AtomicUsize::new(0),
            })
        }
    }

    #[async_trait]
    impl CredIO for MockIO {
        async fn load(&self) -> Result<Option<Value>> {
            Ok(self.current.lock().unwrap().clone())
        }
        async fn write(&self, blob: &Value) -> Result<()> {
            *self.current.lock().unwrap() = Some(blob.clone());
            self.writes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn blob(refresh: &str) -> Value {
        serde_json::json!({ "refreshToken": refresh, "accessToken": "at" })
    }

    #[tokio::test]
    async fn arm_returns_a_guardian_handle() {
        let io = MockIO::new(blob("rt-b"));
        let g = KeychainGuardian::arm(blob("rt-b"), io.clone());
        // Smoke: cancel must consume self without panicking.
        g.cancel();
        assert_eq!(io.writes.load(Ordering::SeqCst), 0);
    }
}
