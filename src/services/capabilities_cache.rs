use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use crate::domain::printer::PrinterCapabilities;

struct Entry {
    caps: PrinterCapabilities,
    cached_at: Instant,
}

/// In-memory TTL cache for printer capabilities. Shared via `Arc`; concurrent-read safe.
pub struct CapabilitiesCache {
    store: RwLock<HashMap<String, Entry>>,
    ttl: Duration,
}

impl CapabilitiesCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
            ttl,
        }
    }

    pub async fn get(&self, printer: &str) -> Option<PrinterCapabilities> {
        let store = self.store.read().await;
        store.get(printer).and_then(|e| {
            if e.cached_at.elapsed() < self.ttl {
                Some(e.caps.clone())
            } else {
                None
            }
        })
    }

    pub async fn set(&self, printer: &str, caps: PrinterCapabilities) {
        let mut store = self.store.write().await;
        store.insert(
            printer.to_owned(),
            Entry {
                caps,
                cached_at: Instant::now(),
            },
        );
    }

    pub async fn invalidate(&self, printer: &str) {
        self.store.write().await.remove(printer);
    }
}
