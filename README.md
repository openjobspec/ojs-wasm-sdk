# OJS WASM SDK

> 🚀 **Status: Alpha** — Core client functionality is implemented. API may change.

A WebAssembly OJS client for browsers, Service Workers, and edge runtimes — compiled from Rust via `wasm-bindgen`.

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) 1.75+
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)

### Build

```bash
# Build the WASM package (output in pkg/)
make build

# Or directly with wasm-pack
wasm-pack build --target web
```

### CDN / npm Distribution

```bash
# Build for npm
wasm-pack build --target bundler --scope openjobspec

# Publish to npm
cd pkg && npm publish --access public
```

Once published, install from npm:

```bash
npm install @openjobspec/wasm
```

Or load directly from a CDN (after publishing):

```html
<script type="module">
  import init, { OJSClient } from 'https://unpkg.com/@openjobspec/wasm/ojs_wasm_sdk.js';
  await init();
</script>
```

---

## Browser Usage

`OJSClient` uses `window.fetch` and is designed for standard browser contexts.

```html
<script type="module">
  import init, { OJSClient } from './pkg/ojs_wasm_sdk.js';

  await init();

  const client = new OJSClient('http://localhost:8080');

  // Enqueue a job
  const job = await client.enqueue('email.send', ['user@example.com', 'Hello!']);
  console.log('Created job:', job.id);

  // Get job status
  const status = await client.get_job(job.id);
  console.log('Job state:', status.state);

  // Batch enqueue
  const jobs = await client.enqueue_batch([
    { type: 'email.send', args: ['a@b.com'] },
    { type: 'report.generate', args: [42] },
  ]);

  // Cancel a job
  await client.cancel_job(job.id);

  // Health check
  const health = await client.health();
  console.log('Server status:', health.status);
</script>
```

### Browser API

| Method | Description |
|--------|-------------|
| `new OJSClient(url)` | Create a client pointing at an OJS server |
| `enqueue(type, args)` | Enqueue a single job |
| `enqueue_batch(jobs)` | Batch enqueue multiple jobs |
| `get_job(id)` | Get job details by ID |
| `cancel_job(id)` | Cancel a job by ID |
| `health()` | Server health check |

---

## Service Worker Integration

`ServiceWorkerClient` uses the global `fetch()` (no `window` dependency) and adds background sync and push notification support.

### Basic Usage

```js
// sw.js — inside your Service Worker
import init, { ServiceWorkerClient } from '@openjobspec/wasm';

await init();

const client = new ServiceWorkerClient('https://api.example.com');

// Enqueue a job (same API as OJSClient)
const job = await client.enqueue('email.send', ['user@example.com']);
```

### Background Sync (Offline Enqueueing)

Register a job for deferred enqueue when the device is offline. The browser will replay it once connectivity is restored.

```js
// In your main page (or SW registration script):
const client = new ServiceWorkerClient('https://api.example.com');

// Queue a job for background sync
const tag = await client.register_sync('email.send', ['user@example.com']);
```

Then handle the `sync` event in your Service Worker:

```js
// sw.js
self.addEventListener('sync', (event) => {
  if (event.tag.startsWith('ojs-enqueue-')) {
    event.waitUntil(client.process_sync(event.tag));
  }
});
```

### Push Notifications for Job Completion

Show a notification when a push message reports a completed job:

```js
// sw.js
self.addEventListener('push', (event) => {
  const data = event.data.json();
  // data = { job_id: "...", job_type: "email.send", state: "completed" }
  event.waitUntil(
    client.notify_job_completed(data.job_id, data.job_type, data.state)
  );
});
```

### Service Worker API

| Method | Description |
|--------|-------------|
| `new ServiceWorkerClient(url)` | Create a client for the SW global scope |
| `enqueue(type, args)` | Enqueue a single job |
| `enqueue_batch(jobs)` | Batch enqueue multiple jobs |
| `get_job(id)` | Get job details by ID |
| `cancel_job(id)` | Cancel a job by ID |
| `health()` | Server health check |
| `register_sync(type, args)` | Register a job for Background Sync |
| `process_sync(tag)` | Process a pending sync tag |
| `notify_job_completed(id, type, state)` | Show a push notification |

---

## Edge Runtime Usage

Edge clients use the global `fetch()` and work in any environment without a `window` object.

### Generic Edge Client

Works in **any** runtime with a global `fetch` (Service Workers, Bun, Node 18+, etc.):

```js
import { EdgeClient } from '@openjobspec/wasm';

const client = new EdgeClient('https://ojs.example.com');
const job = await client.enqueue('email.send', ['user@example.com']);
```

### Cloudflare Workers

```js
import init, { CloudflareClient } from '@openjobspec/wasm';

export default {
  async fetch(request, env, ctx) {
    await init();
    const client = new CloudflareClient('https://ojs.example.com');

    // Standard enqueue
    const job = await client.enqueue('email.send', ['user@example.com']);

    // Fire-and-forget enqueue (non-blocking via ctx.waitUntil)
    client.enqueue_with_wait_until(ctx, 'analytics.track', [request.url]);

    return Response.json(job);
  }
};
```

### Deno Deploy

```ts
import init, { DenoClient } from '@openjobspec/wasm';

await init();

Deno.serve(async (_req) => {
  // Create from explicit URL
  const client = new DenoClient('https://ojs.example.com');

  // Or read OJS_URL from Deno.env automatically
  // const client = DenoClient.from_env();

  const job = await client.enqueue('email.send', ['user@example.com']);
  return Response.json(job);
});
```

### Vercel Edge Functions

```ts
import init, { VercelEdgeClient } from '@openjobspec/wasm';
import { waitUntil } from '@vercel/functions';

export const config = { runtime: 'edge' };

export default async function handler(req: Request) {
  await init();

  // Create from explicit URL
  const client = new VercelEdgeClient('https://ojs.example.com');

  // Or read OJS_URL from process.env automatically
  // const client = VercelEdgeClient.from_env();

  // Fire-and-forget enqueue via Vercel's waitUntil
  client.enqueue_with_wait_until(waitUntil, 'analytics.track', [req.url]);

  const job = await client.enqueue('email.send', ['user@example.com']);
  return Response.json(job);
}
```

### Edge API

All edge clients share the same base methods:

| Method | Description |
|--------|-------------|
| `new <Client>(url)` | Create a client for the target runtime |
| `enqueue(type, args)` | Enqueue a single job |
| `enqueue_batch(jobs)` | Batch enqueue multiple jobs |
| `get_job(id)` | Get job details by ID |
| `cancel_job(id)` | Cancel a job by ID |
| `health()` | Server health check |

Runtime-specific extras:

| Client | Method | Description |
|--------|--------|-------------|
| `CloudflareClient` | `enqueue_with_wait_until(ctx, type, args)` | Fire-and-forget via `ctx.waitUntil` |
| `DenoClient` | `DenoClient.from_env()` | Create client from `OJS_URL` env var |
| `VercelEdgeClient` | `VercelEdgeClient.from_env()` | Create client from `OJS_URL` env var |
| `VercelEdgeClient` | `enqueue_with_wait_until(fn, type, args)` | Fire-and-forget via Vercel `waitUntil` |

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│  ojs-wasm-sdk                                   │
│                                                 │
│  ┌───────────────────────────────────────────┐  │
│  │  Rust core (wasm32-unknown-unknown)       │  │
│  │                                           │  │
│  │  OJSClient            (window.fetch)      │  │
│  │  ServiceWorkerClient  (global fetch)      │  │
│  │  EdgeClient           (global fetch)      │  │
│  │  CloudflareClient     (+ waitUntil)       │  │
│  │  DenoClient           (+ Deno.env)        │  │
│  │  VercelEdgeClient     (+ process.env)     │  │
│  └───────────────────────────────────────────┘  │
│                                                 │
│  ┌───────────────────────────────────────────┐  │
│  │  JavaScript bindings (wasm-bindgen)       │  │
│  └───────────────────────────────────────────┘  │
│                                                 │
│  ┌───────────────────────────────────────────┐  │
│  │  TypeScript type definitions (.d.ts)      │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

## Contributing

If you're interested in contributing to the WASM SDK, please:

1. Join the [discussion](https://github.com/openjobspec/spec/discussions)
2. Review the [OJS Rust SDK](../ojs-rust-sdk/) which will serve as the WASM core
3. Check the [Roadmap](../ROADMAP.md) for timeline updates

## License

[Apache License 2.0](../LICENSE)
