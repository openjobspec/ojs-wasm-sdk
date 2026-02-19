use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Job state (mirrors ojs-rust-sdk JobState)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Pending,
    Scheduled,
    Available,
    Active,
    Completed,
    Retryable,
    Cancelled,
    Discarded,
}

// ---------------------------------------------------------------------------
// Enqueue request (POST /ojs/v1/jobs)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct EnqueueRequest {
    #[serde(rename = "type")]
    pub job_type: String,
    pub args: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Batch request (POST /ojs/v1/jobs/batch)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct BatchRequest {
    pub jobs: Vec<EnqueueRequest>,
}

// ---------------------------------------------------------------------------
// Job response (returned from server)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResponse {
    pub job: Job,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    #[serde(rename = "type")]
    pub job_type: String,
    #[serde(default = "default_queue")]
    pub queue: String,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default)]
    pub priority: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<JobState>,
    #[serde(default)]
    pub attempt: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enqueued_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

fn default_queue() -> String {
    "default".to_string()
}

// ---------------------------------------------------------------------------
// Batch response (POST /ojs/v1/jobs/batch)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct BatchResponse {
    pub jobs: Vec<Job>,
    #[serde(default)]
    pub count: usize,
}

// ---------------------------------------------------------------------------
// Health response (GET /ojs/v1/health)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
}
