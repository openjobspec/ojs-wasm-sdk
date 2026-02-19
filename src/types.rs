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
// Enqueue options
// ---------------------------------------------------------------------------

/// Optional settings for job enqueue requests.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct EnqueueOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_until: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Enqueue request (POST /ojs/v1/jobs)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct EnqueueRequest {
    #[serde(rename = "type")]
    pub job_type: String,
    pub args: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<EnqueueOptions>,
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
    pub tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
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

// ---------------------------------------------------------------------------
// Workflow types
// ---------------------------------------------------------------------------

/// Workflow lifecycle states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Workflow status as returned by the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub workflow_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<WorkflowState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<WorkflowMetadata>,
}

/// Metadata about a workflow's progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub job_count: u32,
    #[serde(default)]
    pub completed_count: u32,
    #[serde(default)]
    pub failed_count: u32,
}
