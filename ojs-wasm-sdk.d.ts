/**
 * TypeScript type definitions for @openjobspec/wasm
 *
 * These definitions describe the public API exposed by the WASM module
 * after calling `init()`. They are hand-written to provide richer types
 * than the auto-generated wasm-bindgen output.
 */

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/**
 * Initialize the WASM module. Must be called once before using any client.
 *
 * @param input - Optional path or URL to the .wasm file. Defaults to the
 *                co-located `ojs_wasm_sdk_bg.wasm`.
 */
export default function init(input?: string | URL | Request): Promise<void>;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/** OJS job state. */
export type JobState =
  | 'pending'
  | 'scheduled'
  | 'available'
  | 'active'
  | 'completed'
  | 'retryable'
  | 'cancelled'
  | 'discarded';

/** A job as returned by the OJS server. */
export interface Job {
  id: string;
  type: string;
  queue: string;
  args: unknown[];
  priority: number;
  state?: JobState;
  attempt: number;
  tags?: string[];
  meta?: Record<string, unknown>;
  created_at?: string;
  enqueued_at?: string;
  started_at?: string;
  completed_at?: string;
}

/** Options for enqueuing a job. */
export interface EnqueueOptions {
  queue?: string;
  priority?: number;
  timeout_ms?: number;
  delay_until?: string;
  tags?: string[];
}

/** A job specification for batch enqueue or workflow steps. */
export interface JobSpec {
  type: string;
  args: unknown[];
  options?: EnqueueOptions;
}

/** Server health response. */
export interface HealthResponse {
  status: string;
  version?: string;
  uptime_seconds?: number;
}

/** Workflow lifecycle state. */
export type WorkflowState =
  | 'pending'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled';

/** Workflow status as returned by the server. */
export interface WorkflowStatus {
  id: string;
  type: 'chain' | 'group' | 'batch';
  name?: string;
  state?: WorkflowState;
  metadata?: {
    created_at?: string;
    started_at?: string;
    completed_at?: string;
    job_count: number;
    completed_count: number;
    failed_count: number;
  };
}

/** Batch workflow callback definitions. */
export interface BatchCallbacks {
  on_complete?: JobSpec;
  on_success?: JobSpec;
  on_failure?: JobSpec;
}

/** Queue information. */
export interface QueueInfo {
  name: string;
  paused: boolean;
  depth: number;
}

/** Queue statistics. */
export interface QueueStats {
  name: string;
  pending: number;
  active: number;
  completed: number;
  failed: number;
  paused: boolean;
}

// ---------------------------------------------------------------------------
// Middleware
// ---------------------------------------------------------------------------

/** Request object passed through the middleware chain. */
export interface MiddlewareRequest {
  method: string;
  url: string;
  headers: Record<string, string>;
  body: unknown;
}

/** Middleware chain for intercepting requests. */
export class MiddlewareChain {
  constructor();
  add(name: string, handler: (req: MiddlewareRequest) => MiddlewareRequest): void;
  remove(name: string): void;
  list(): string[];
  apply(request: MiddlewareRequest): MiddlewareRequest;
}

/** Create a request object for middleware processing. */
export function create_request(
  method: string,
  url: string,
  body: unknown,
): MiddlewareRequest;

// ---------------------------------------------------------------------------
// Retry Policy
// ---------------------------------------------------------------------------

/** Retry policy configuration for job enqueue requests. */
export class RetryPolicy {
  /** Create an exponential backoff retry policy. */
  static exponential(
    max_attempts: number,
    initial_delay_ms: number,
    max_delay_ms: number,
  ): RetryPolicy;

  /** Create a fixed-interval retry policy. */
  static fixed(max_attempts: number, delay_ms: number): RetryPolicy;

  /** Create a linear backoff retry policy. */
  static linear(
    max_attempts: number,
    initial_delay_ms: number,
    max_delay_ms: number,
  ): RetryPolicy;

  /** Convert to a JS object suitable for use in enqueue options. */
  to_object(): Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// Workflow builders (free functions)
// ---------------------------------------------------------------------------

/**
 * Create a chain workflow (sequential execution).
 * Each step runs after the previous one completes.
 */
export function chain(steps: JobSpec[]): Record<string, unknown>;

/**
 * Create a group workflow (parallel execution).
 * All jobs run concurrently.
 */
export function group(jobs: JobSpec[]): Record<string, unknown>;

/**
 * Create a batch workflow (parallel with callbacks).
 * Like a group but fires callbacks based on collective outcome.
 */
export function batch(
  jobs: JobSpec[],
  callbacks: BatchCallbacks,
): Record<string, unknown>;

// ---------------------------------------------------------------------------
// OJSClient (browser)
// ---------------------------------------------------------------------------

/** Browser OJS client using `window.fetch`. */
export class OJSClient {
  constructor(url: string);

  /** Enqueue a single job. */
  enqueue(type: string, args: unknown[]): Promise<Job>;

  /** Enqueue a single job with options. */
  enqueue_with_options(
    type: string,
    args: unknown[],
    options: EnqueueOptions,
  ): Promise<Job>;

  /** Batch enqueue multiple jobs. */
  enqueue_batch(jobs: JobSpec[]): Promise<Job[]>;

  /** Get a job by ID. */
  get_job(id: string): Promise<Job>;

  /** Cancel a job by ID. */
  cancel_job(id: string): Promise<Job>;

  /** Create and start a workflow. */
  workflow(definition: Record<string, unknown>): Promise<WorkflowStatus>;

  /** Get the status of a workflow. */
  get_workflow(workflow_id: string): Promise<WorkflowStatus>;

  /** Server health check. */
  health(): Promise<HealthResponse>;

  /** List all queues. */
  list_queues(): Promise<QueueInfo[]>;

  /** Get statistics for a specific queue. */
  queue_stats(queue_name: string): Promise<QueueStats>;

  /** Pause a queue. */
  pause_queue(queue_name: string): Promise<void>;

  /** Resume a paused queue. */
  resume_queue(queue_name: string): Promise<void>;
}

// ---------------------------------------------------------------------------
// ServiceWorkerClient
// ---------------------------------------------------------------------------

/** OJS client for Service Worker contexts. Uses global `fetch()`. */
export class ServiceWorkerClient {
  constructor(url: string);

  enqueue(type: string, args: unknown[]): Promise<Job>;
  enqueue_with_options(
    type: string,
    args: unknown[],
    options: EnqueueOptions,
  ): Promise<Job>;
  enqueue_batch(jobs: JobSpec[]): Promise<Job[]>;
  get_job(id: string): Promise<Job>;
  cancel_job(id: string): Promise<Job>;
  workflow(definition: Record<string, unknown>): Promise<WorkflowStatus>;
  get_workflow(workflow_id: string): Promise<WorkflowStatus>;
  health(): Promise<HealthResponse>;

  /** Register a job for Background Sync (offline enqueueing). */
  register_sync(type: string, args: unknown[]): Promise<string>;

  /** Process a pending sync tag in the `sync` event handler. */
  process_sync(tag: string): Promise<Job>;

  /** Show a push notification when a job completes. */
  notify_job_completed(
    job_id: string,
    job_type: string,
    state: string,
  ): Promise<boolean>;
}

// ---------------------------------------------------------------------------
// EdgeClient (generic)
// ---------------------------------------------------------------------------

/** Generic edge-runtime OJS client. Works anywhere with global `fetch()`. */
export class EdgeClient {
  constructor(url: string);

  /** Create a client with Bearer token authentication. */
  static with_auth(url: string, api_key: string): EdgeClient;

  enqueue(type: string, args: unknown[]): Promise<Job>;
  enqueue_with_options(
    type: string,
    args: unknown[],
    options: EnqueueOptions,
  ): Promise<Job>;
  enqueue_batch(jobs: JobSpec[]): Promise<Job[]>;
  get_job(id: string): Promise<Job>;
  cancel_job(id: string): Promise<Job>;
  workflow(definition: Record<string, unknown>): Promise<WorkflowStatus>;
  get_workflow(workflow_id: string): Promise<WorkflowStatus>;
  health(): Promise<HealthResponse>;
}

// ---------------------------------------------------------------------------
// CloudflareClient
// ---------------------------------------------------------------------------

/** OJS client for Cloudflare Workers with `ctx.waitUntil` support. */
export class CloudflareClient {
  constructor(url: string);
  static with_auth(url: string, api_key: string): CloudflareClient;

  enqueue(type: string, args: unknown[]): Promise<Job>;
  enqueue_with_options(
    type: string,
    args: unknown[],
    options: EnqueueOptions,
  ): Promise<Job>;
  enqueue_batch(jobs: JobSpec[]): Promise<Job[]>;
  get_job(id: string): Promise<Job>;
  cancel_job(id: string): Promise<Job>;
  workflow(definition: Record<string, unknown>): Promise<WorkflowStatus>;
  get_workflow(workflow_id: string): Promise<WorkflowStatus>;
  health(): Promise<HealthResponse>;

  /**
   * Fire-and-forget enqueue via Cloudflare's `ctx.waitUntil`.
   * The HTTP request runs in the background after the response is sent.
   */
  enqueue_with_wait_until(
    ctx: { waitUntil(promise: Promise<unknown>): void },
    type: string,
    args: unknown[],
  ): void;
}

// ---------------------------------------------------------------------------
// DenoClient
// ---------------------------------------------------------------------------

/** OJS client for Deno Deploy with `Deno.env` integration. */
export class DenoClient {
  constructor(url: string);
  static with_auth(url: string, api_key: string): DenoClient;

  /** Create a client from the `OJS_URL` environment variable. */
  static from_env(): DenoClient;

  enqueue(type: string, args: unknown[]): Promise<Job>;
  enqueue_with_options(
    type: string,
    args: unknown[],
    options: EnqueueOptions,
  ): Promise<Job>;
  enqueue_batch(jobs: JobSpec[]): Promise<Job[]>;
  get_job(id: string): Promise<Job>;
  cancel_job(id: string): Promise<Job>;
  workflow(definition: Record<string, unknown>): Promise<WorkflowStatus>;
  get_workflow(workflow_id: string): Promise<WorkflowStatus>;
  health(): Promise<HealthResponse>;
}

// ---------------------------------------------------------------------------
// VercelEdgeClient
// ---------------------------------------------------------------------------

/** OJS client for Vercel Edge Functions with `waitUntil` support. */
export class VercelEdgeClient {
  constructor(url: string);
  static with_auth(url: string, api_key: string): VercelEdgeClient;

  /** Create a client from the `OJS_URL` environment variable. */
  static from_env(): VercelEdgeClient;

  enqueue(type: string, args: unknown[]): Promise<Job>;
  enqueue_with_options(
    type: string,
    args: unknown[],
    options: EnqueueOptions,
  ): Promise<Job>;
  enqueue_batch(jobs: JobSpec[]): Promise<Job[]>;
  get_job(id: string): Promise<Job>;
  cancel_job(id: string): Promise<Job>;
  workflow(definition: Record<string, unknown>): Promise<WorkflowStatus>;
  get_workflow(workflow_id: string): Promise<WorkflowStatus>;
  health(): Promise<HealthResponse>;

  /**
   * Fire-and-forget enqueue via Vercel's `waitUntil`.
   */
  enqueue_with_wait_until(
    waitUntilFn: (promise: Promise<unknown>) => void,
    type: string,
    args: unknown[],
  ): void;
}
