use dashmap::DashMap;
use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};

use super::redis_backend::{redis_url_from_env, RedisCache};
use crate::common::AppResult;

#[derive(Clone, Debug)]
pub(crate) struct CacheService {
    backend: Arc<CacheBackend>,
}

#[derive(Debug)]
enum CacheBackend {
    Redis(RedisCache),
    // TODO: Remove this fallback once a cache is a required dependency
    // rather than optional. Without one, cached values live only in this
    // process's memory, so a server crash or restart silently drops
    // everything held here, including websocket pub-sub subscriptions.
    Memory(MemoryCache),
}

#[derive(Debug, Default)]
struct MemoryCache {
    values: DashMap<String, (String, Instant)>,
    members: DashMap<String, HashSet<String>>,
}

impl CacheService {
    pub(crate) fn from_env() -> Self {
        let Some(redis_url) = redis_url_from_env() else {
            tracing::info!(
                "Redis cache is not configured; using an in-memory cache."
            );
            return Self::memory();
        };

        match RedisCache::connect(&redis_url) {
            Ok(redis) => Self {
                backend: Arc::new(CacheBackend::Redis(redis)),
            },
            Err(error) => {
                tracing::warn!(
                    "failed to initialize Redis cache at {redis_url}: {error}; using an in-memory cache"
                );
                Self::memory()
            }
        }
    }

    fn memory() -> Self {
        Self {
            backend: Arc::new(CacheBackend::Memory(MemoryCache::default())),
        }
    }

    pub(crate) async fn get(&self, key: &str) -> AppResult<Option<String>> {
        match self.backend.as_ref() {
            CacheBackend::Redis(redis) => redis.get(key).await,
            CacheBackend::Memory(memory) => Ok(memory
                .values
                .get(key)
                .filter(|entry| entry.1 > Instant::now())
                .map(|entry| entry.0.clone())),
        }
    }

    pub(crate) async fn set(
        &self,
        key: String,
        value: String,
        expiry: Duration,
    ) -> AppResult<()> {
        match self.backend.as_ref() {
            CacheBackend::Redis(redis) => {
                redis.set(&key, &value, expiry.as_secs()).await
            }
            CacheBackend::Memory(memory) => {
                memory.values.insert(key, (value, Instant::now() + expiry));
                Ok(())
            }
        }
    }

    pub(crate) async fn add_member(
        &self,
        key: String,
        value: String,
    ) -> AppResult<()> {
        match self.backend.as_ref() {
            CacheBackend::Redis(redis) => redis.add_member(&key, &value).await,
            CacheBackend::Memory(memory) => {
                memory.members.entry(key).or_default().insert(value);
                Ok(())
            }
        }
    }

    pub(crate) async fn remove_member(
        &self,
        key: String,
        value: &str,
    ) -> AppResult<()> {
        match self.backend.as_ref() {
            CacheBackend::Redis(redis) => {
                redis.remove_member(&key, value).await
            }
            CacheBackend::Memory(memory) => {
                let remove_key =
                    if let Some(mut members) = memory.members.get_mut(&key) {
                        members.remove(value);
                        members.is_empty()
                    } else {
                        false
                    };
                if remove_key {
                    memory.members.remove(&key);
                }
                Ok(())
            }
        }
    }

    pub(crate) async fn members(&self, key: String) -> AppResult<Vec<String>> {
        match self.backend.as_ref() {
            CacheBackend::Redis(redis) => redis.members(&key).await,
            CacheBackend::Memory(memory) => Ok(memory
                .members
                .get(&key)
                .map(|members| members.iter().cloned().collect())
                .unwrap_or_default()),
        }
    }
}
