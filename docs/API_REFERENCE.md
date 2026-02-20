# API Reference

Complete API documentation for the OJS WASM SDK.

## Initialization

### `init(input?)`

Initialize the WASM module. Must be called once before using any client.

```typescript
import init from '@openjobspec/wasm';

await init();                       // default .wasm location
await init('/path/to/wasm');        // custom .wasm path
await init(new URL('./wasm', import.meta.url));
```

**Parameters:**
- `input` (optional): Path, URL, or Request to the `.wasm` file

**Returns:** `Promise<void>`

---

## Data Types

### `Job`

A job as returned by the OJS server.

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string` | Unique job identifier (UUIDv7) |
| `type` | `string` | Dot-namespaced job type (e.g., `email.send`) |
| `queue` | `string` | Target queue name |
| `args` | `unknown[]` | Positional job arguments |
| `priority` | `number` | Job priority (lower = higher priority) |
| `state` | `JobState` | Current lifecycle state |
| `attempt` | `number` | Current attempt number |
| `tags` | `string[]` | Optional tags for filtering |
| `meta` | `Record<string, unknown>` | Optional metadata |
| `created_at` | `string` | ISO 8601 creation timestamp |
| `enqueued_at` | `string` | ISO 8601 enqueue timestamp |
| `started_at` | `string` | ISO 8601 start timestamp |
| `completed_at` | `string` | ISO 8601 completion timestamp |

### `JobState`

```typescript
type JobState =
  | 'pending' | 'scheduled' | 'available' | 'active'
  | 'completed' | 'retryable' | 'cancelled' | 'discarded';
```

### `EnqueueOptions`

| Field | Type | Description |
|-------|------|-------------|
| `queue` | `string` | Target queue (default: `"default"`) |
| `priority` | `number` | Priority (lower = higher priority) |
| `timeout_ms` | `number` | Processing timeout in milliseconds |
| `delay_until` | `string` | ISO 8601 timestamp to delay until |
| `tags` | `string[]` | Tags for filtering |

### `JobSpec`

Job specification for batch enqueue and workflow steps.

| Field | Type | Description |
|-------|------|-------------|
| `type` | `string` | Job type |
| `args` | `unknown[]` | Job arguments |
| `options` | `EnqueueOptions` | Optional enqueue options |

### `WorkflowStatus`

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string` | Workflow identifier |
| `type` | `'chain' \| 'group' \| 'batch'` | Workflow type |
| `name` | `string` | Optional workflow name |
| `state` | `WorkflowState` | Lifecycle state |
| `metadata.job_count` | `number` | Total jobs in workflow |
| `metadata.completed_count` | `number` | Completed jobs |
| `metadata.failed_count` | `number` | Failed jobs |

### `HealthResponse`

| Field | Type | Description |
|-------|------|-------------|
| `status` | `string` | Server status (e.g., `"healthy"`) |
| `version` | `string` | Server version |
| `uptime_seconds` | `number` | Server uptime |

### `QueueInfo`

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Queue name |
| `paused` | `boolean` | Whether the queue is paused |
| `depth` | `number` | Number of jobs in the queue |

### `QueueStats`

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Queue name |
| `pending` | `number` | Pending job count |
| `active` | `number` | Active job count |
| `completed` | `number` | Completed job count |
| `failed` | `number` | Failed job count |
| `paused` | `boolean` | Whether the queue is paused |

---

## Client Classes

All clients share the same core API. The difference is how they integrate with their runtime environment.

### `OJSClient` (Browser)

Uses `window.fetch`. For browser SPAs, forms, and web pages.

```typescript
const client = new OJSClient('http://localhost:8080');
```

### `EdgeClient` (Generic)

Uses global `fetch()`. Works in Node.js 18+, Bun, and any runtime with global fetch.

```typescript
const client = new EdgeClient('http://localhost:8080');
const client = EdgeClient.with_auth('https://ojs.example.com', 'api-key');
```

### `CloudflareClient`

Extends EdgeClient with `ctx.waitUntil()` support.

```typescript
const client = new CloudflareClient(env.OJS_URL);
const client = CloudflareClient.with_auth(env.OJS_URL, env.OJS_API_KEY);

// Fire-and-forget enqueue
client.enqueue_with_wait_until(ctx, 'job.type', [args]);
```

### `DenoClient`

Integrates with `Deno.env` for environment-based configuration.

```typescript
const client = new DenoClient('http://localhost:8080');
const client = DenoClient.with_auth(url, apiKey);
const client = DenoClient.from_env(); // reads OJS_URL, OJS_API_KEY
```

### `VercelEdgeClient`

Supports Vercel's `waitUntil` function for background work.

```typescript
const client = new VercelEdgeClient(process.env.OJS_URL!);
const client = VercelEdgeClient.from_env();

// Fire-and-forget via Vercel's waitUntil
import { waitUntil } from '@vercel/functions';
client.enqueue_with_wait_until(waitUntil, 'job.type', [args]);
```

### `ServiceWorkerClient`

For service worker contexts with Background Sync and push notification support.

```typescript
const client = new ServiceWorkerClient('https://ojs.example.com');

// Register a job for Background Sync (offline enqueueing)
const tag = await client.register_sync('email.send', [to, subject]);

// Process a pending sync tag
const job = await client.process_sync(tag);

// Show a push notification when a job completes
await client.notify_job_completed(jobId, jobType, state);
```

---

## Shared Client Methods

All clients implement these methods:

### `enqueue(type, args)`

Enqueue a single job.

```typescript
const job = await client.enqueue('email.send', ['user@example.com', 'Hello']);
```

**Returns:** `Promise<Job>`

### `enqueue_with_options(type, args, options)`

Enqueue a job with options.

```typescript
const job = await client.enqueue_with_options('report.generate', ['monthly'], {
  queue: 'reports',
  priority: 5,
  tags: ['finance'],
});
```

**Returns:** `Promise<Job>`

### `enqueue_batch(jobs)`

Batch enqueue multiple jobs in a single request.

```typescript
const jobs = await client.enqueue_batch([
  { type: 'email.send', args: ['a@example.com'] },
  { type: 'email.send', args: ['b@example.com'] },
]);
```

**Returns:** `Promise<Job[]>`

### `get_job(id)`

Retrieve a job by its ID.

```typescript
const job = await client.get_job('01234567-89ab-cdef-0123-456789abcdef');
```

**Returns:** `Promise<Job>`

### `cancel_job(id)`

Cancel a job. Only jobs in cancellable states (`available`, `scheduled`, `retryable`) can be cancelled.

```typescript
const cancelled = await client.cancel_job(jobId);
```

**Returns:** `Promise<Job>`

### `workflow(definition)`

Create and start a workflow.

```typescript
import { chain } from '@openjobspec/wasm';

const definition = chain([
  { type: 'step.one', args: [] },
  { type: 'step.two', args: [] },
]);
const status = await client.workflow(definition);
```

**Returns:** `Promise<WorkflowStatus>`

### `get_workflow(workflow_id)`

Get the status of a workflow.

```typescript
const status = await client.get_workflow(workflowId);
```

**Returns:** `Promise<WorkflowStatus>`

### `health()`

Server health check.

```typescript
const health = await client.health();
// { status: 'healthy', version: '1.0.0', uptime_seconds: 3600 }
```

**Returns:** `Promise<HealthResponse>`

### `list_queues()`

List all queues.

**Returns:** `Promise<QueueInfo[]>`

### `queue_stats(queue_name)`

Get statistics for a specific queue.

**Returns:** `Promise<QueueStats>`

### `pause_queue(queue_name)` / `resume_queue(queue_name)`

Pause or resume a queue.

**Returns:** `Promise<void>`

---

## Workflow Builders

Free functions for constructing workflow definitions.

### `chain(steps)`

Create a sequential workflow. Each step runs after the previous one completes.

```typescript
import { chain } from '@openjobspec/wasm';
const definition = chain([
  { type: 'data.fetch', args: ['url'] },
  { type: 'data.transform', args: ['csv'] },
]);
```

### `group(jobs)`

Create a parallel workflow. All jobs run concurrently.

```typescript
import { group } from '@openjobspec/wasm';
const definition = group([
  { type: 'resize', args: ['800x600'] },
  { type: 'resize', args: ['400x300'] },
]);
```

### `batch(jobs, callbacks)`

Create a parallel workflow with callbacks. Fires callbacks based on collective outcome.

```typescript
import { batch } from '@openjobspec/wasm';
const definition = batch(
  [
    { type: 'process', args: ['item-1'] },
    { type: 'process', args: ['item-2'] },
  ],
  {
    on_complete: { type: 'notify', args: ['batch done'] },
    on_success: { type: 'cleanup', args: [] },
    on_failure: { type: 'alert', args: ['batch failed'] },
  },
);
```

---

## Middleware

### `MiddlewareChain`

Intercept and modify requests before they are sent.

```typescript
import { MiddlewareChain, create_request } from '@openjobspec/wasm';

const mw = new MiddlewareChain();
mw.add('auth', (req) => {
  req.headers['Authorization'] = 'Bearer token';
  return req;
});
mw.list();           // ['auth']
mw.remove('auth');
```

---

## Retry Policies

### `RetryPolicy`

Configure retry behavior for enqueued jobs.

```typescript
import { RetryPolicy } from '@openjobspec/wasm';

const exponential = RetryPolicy.exponential(5, 1000, 60000);
const fixed = RetryPolicy.fixed(3, 5000);
const linear = RetryPolicy.linear(5, 1000, 30000);

// Use with enqueue options
const job = await client.enqueue_with_options('job.type', [], {
  ...exponential.to_object(),
});
```
