use crate::types::SectorSummary;
use std::sync::Mutex;

pub struct SectorCache {
    data: Mutex<Option<CacheEntry>>,
}

struct CacheEntry {
    sectors: Vec<SectorSummary>,
    cached_at: std::time::Instant,
}

const CACHE_TTL_SECS: u64 = 15 * 60; // 15 minutes

impl SectorCache {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(None),
        }
    }

    pub fn get(&self) -> Option<Vec<SectorSummary>> {
        let guard = self.data.lock().ok()?;
        let entry = guard.as_ref()?;
        if entry.cached_at.elapsed().as_secs() < CACHE_TTL_SECS {
            Some(entry.sectors.clone())
        } else {
            None
        }
    }

    pub fn set(&self, sectors: Vec<SectorSummary>) {
        if let Ok(mut guard) = self.data.lock() {
            *guard = Some(CacheEntry {
                sectors,
                cached_at: std::time::Instant::now(),
            });
        }
    }

    pub fn get_even_if_expired(&self) -> Option<Vec<SectorSummary>> {
        let guard = self.data.lock().ok()?;
        guard.as_ref().map(|entry| entry.sectors.clone())
    }
}
