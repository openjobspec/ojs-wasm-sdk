//! # Queue Management
//!
//! Queue inspection and management operations for the WASM SDK.
//!
//! # Example
//!
//! ```js
//! import init, { OJSClient } from '@openjobspec/wasm';
//!
//! const client = new OJSClient("http://localhost:8080");
//!
//! // List queues
//! const queues = await client.list_queues();
//!
//! // Get queue stats
//! const stats = await client.queue_stats("default");
//!
//! // Pause/resume a queue
//! await client.pause_queue("default");
//! await client.resume_queue("default");
//! ```

use crate::error::{OjsWasmError, Result};
use crate::transport;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Queue information returned by the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueInfo {
    pub name: String,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub depth: u64,
}

/// Queue statistics returned by the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub name: String,
    #[serde(default)]
    pub pending: u64,
    #[serde(default)]
    pub active: u64,
    #[serde(default)]
    pub completed: u64,
    #[serde(default)]
    pub failed: u64,
    #[serde(default)]
    pub paused: bool,
}

#[derive(Debug, Deserialize)]
struct QueuesResponse {
    queues: Vec<QueueInfo>,
}

#[derive(Debug, Deserialize)]
struct StatsResponse {
    #[serde(flatten)]
    stats: QueueStats,
}

/// Queue management operations.
/// These methods extend OJSClient; call them on the client instance.
pub struct QueueManager {
    base_url: String,
}

impl QueueManager {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    /// List all queues.
    pub async fn list_queues(&self) -> Result<JsValue> {
        let url = format!("{}/queues", self.base_url);
        let resp_text = transport::get(&url).await?;
        let resp: QueuesResponse = serde_json::from_str(&resp_text)?;
        serde_wasm_bindgen::to_value(&resp.queues)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    /// Get statistics for a specific queue.
    pub async fn queue_stats(&self, queue_name: &str) -> Result<JsValue> {
        let url = format!("{}/queues/{}/stats", self.base_url, queue_name);
        let resp_text = transport::get(&url).await?;
        let resp: StatsResponse = serde_json::from_str(&resp_text)?;
        serde_wasm_bindgen::to_value(&resp.stats)
            .map_err(|e| OjsWasmError::Serialization(e.to_string()))
    }

    /// Pause a queue.
    pub async fn pause_queue(&self, queue_name: &str) -> Result<()> {
        let url = format!("{}/queues/{}/pause", self.base_url, queue_name);
        transport::post(&url, "{}").await?;
        Ok(())
    }

    /// Resume a paused queue.
    pub async fn resume_queue(&self, queue_name: &str) -> Result<()> {
        let url = format!("{}/queues/{}/resume", self.base_url, queue_name);
        transport::post(&url, "{}").await?;
        Ok(())
    }
}
