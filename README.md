# OJS WASM SDK

[![CI](https://github.com/openjobspec/ojs-wasm-sdk/actions/workflows/ci.yml/badge.svg)](https://github.com/openjobspec/ojs-wasm-sdk/actions/workflows/ci.yml)

A WebAssembly OJS (Open Job Spec) client for browsers and edge runtimes — compiled from Rust via `wasm-bindgen`.

> **Status: Experimental** — This SDK is functional but has not been battle-tested in production. We welcome feedback and contributions.

## Overview

The OJS WASM SDK lets you enqueue and manage background jobs from any JavaScript environment that supports WebAssembly. It compiles the core OJS types and HTTP logic from Rust to WASM, giving you a small, fast, dependency-free client that works everywhere from browser SPAs to Cloudflare Workers.

### Supported Runtimes

| Runtime | Client | Notes |
|---------|--------|-------|
| **Browser** | `OJSClient` | Uses `window.fetch`; works in any modern browser |
| **Service Worker** | `ServiceWorkerClient` | Global `fetch()`; Background Sync + push notifications |
| **Cloudflare Workers** | `CloudflareClient` | `ctx.waitUntil` for fire-and-forget enqueue |
| **Deno / Deno Deploy** | `DenoClient` | `Deno.env` integration for config |
| **Vercel Edge Functions** | `VercelEdgeClient` | Vercel `waitUntil` support |
| **Node.js 18+** | `EdgeClient` | Via WASM — any runtime with global `fetch()` |
| **Bun** | `EdgeClient` | Uses global `fetch()` |

## Features

- **Browser client** (`OJSClient`) — enqueue jobs from web forms, SPAs, or any browser context
- **Service Worker client** (`ServiceWorkerClient`) — offline-first with Background Sync and push notifications
- **Edge runtime clients** — Cloudflare Workers, Deno Deploy, Vercel Edge Functions
- **Workflow support** — `chain()`, `group()`, `batch()` builder functions
- **Enqueue options** — queue, priority, timeout, delay, tags
- **Middleware chain** — intercept and modify requests before sending
- **Retry policies** — exponential, linear, and fixed backoff configurations
- **Queue management** — list, inspect, pause, and resume queues
- **Small footprint** — compiled with `opt-level = "s"` and LTO; typical gzipped size is under 50 KB
- **No Node.js dependencies** — uses the Web Fetch API exclusively

## Installation

### npm

```bash
npm install @openjobspec/wasm
```

### CDN (unpkg / esm.sh)

```html
<script type="module">
  import init, { OJSClient } from 'https://unpkg.com/@openjobspec/wasm/ojs_wasm_sdk.js';
  await init();
</script>
```

Or via esm.sh for Deno:

```ts
import init, { DenoClient } from 'https://esm.sh/@openjobspec/wasm';
```

### Cargo (as a Rust dependency)

```toml
[dependencies]
ojs-wasm-sdk = "0.1"

# Enable runtime-specific features:
# ojs-wasm-sdk = { version = "0.1", features = ["edge_cloudflare"] }
```

### Build from source

Prerequisites: [Rust 1.75+](https://rustup.rs/) and [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/).

```bash
# Build for browser (ESM, manual init)
make build

# Or directly with wasm-pack
wasm-pack build --target web

# Optimized release build
make build-release

# Build for bundler (webpack, Vite, Rollup)
make build-bundler
```

The compiled output lands in `pkg/`. The key files are:

| File | Purpose |
|------|---------|
| `ojs_wasm_sdk.js` | ESM loader with `init()` default export |
| `ojs_wasm_sdk_bg.wasm` | The compiled WebAssembly binary |
| `ojs_wasm_sdk.d.ts` | Auto-generated TypeScript definitions |

## Quick Start

### Browser

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
</script>
```

### Cloudflare Workers

```js
import init, { CloudflareClient } from '@openjobspec/wasm';

let initialized = false;

export default {
  async fetch(request, env, ctx) {
    if (!initialized) { await init(); initialized = true; }

    const client = new CloudflareClient(env.OJS_URL);
    const job = await client.enqueue('email.send', ['user@example.com']);

    // Fire-and-forget (non-blocking)
    client.enqueue_with_wait_until(ctx, 'analytics.track', [request.url]);

    return Response.json(job);
  }
};
```

### Deno

```ts
import init, { DenoClient } from '@openjobspec/wasm';

await init();
const client = DenoClient.from_env(); // reads OJS_URL

Deno.serve(async (_req) => {
  const job = await client.enqueue('email.send', ['user@example.com']);
  return Response.json(job);
});
```

### Node.js (18+)

```js
import init, { EdgeClient } from '@openjobspec/wasm';

await init();
const client = new EdgeClient('http://localhost:8080');
const job = await client.enqueue('email.send', ['user@example.com']);
console.log(job);
```

---

## Browser Usage

`OJSClient` uses `window.fetch` and is designed for standard browser contexts.

### Basic enqueue

```js
import init, { OJSClient } from './pkg/ojs_wasm_sdk.js';

await init();
const client = new OJSClient('http://localhost:8080');

const job = await client.enqueue('email.send', ['user@example.com', 'Welcome!']);
console.log('Job ID:', job.id);
```

### Enqueue with options

```js
const job = await client.enqueue_with_options('email.send', ['user@example.com'], {
  queue: 'critical',
  priority: 10,
  timeout_ms: 30000,
  tags: ['onboarding', 'welcome'],
});
```

### Batch enqueue

```js
const jobs = await client.enqueue_batch([
  { type: 'email.send', args: ['a@b.com'] },
  { type: 'report.generate', args: [42] },
  { type: 'sms.send', args: ['+1234567890', 'Hello'], options: { queue: 'sms' } },
]);
console.log(`Enqueued ${jobs.length} jobs`);
```

### Cancel a job

```js
const cancelled = await client.cancel_job(job.id);
console.log('Cancelled:', cancelled.state);
```

### Health check

```js
const health = await client.health();
console.log('Server status:', health.status);
```

### Browser API

| Method | Description |
|--------|-------------|
| `new OJSClient(url)` | Create a client pointing at an OJS server |
| `enqueue(type, args)` | Enqueue a single job |
| `enqueue_with_options(type, args, options)` | Enqueue with queue/priority/tags/etc. |
| `enqueue_batch(jobs)` | Batch enqueue multiple jobs |
| `get_job(id)` | Get job details by ID |
| `cancel_job(id)` | Cancel a job by ID |
| `workflow(definition)` | Create and start a workflow |
| `get_workflow(id)` | Get workflow status |
| `health()` | Server health check |

---

## Workflows

The SDK provides three workflow builder functions that mirror the OJS workflow specification.

### chain (sequential)

Each step runs after the previous one completes.

```js
import init, { OJSClient, chain } from './pkg/ojs_wasm_sdk.js';

await init();
const client = new OJSClient('http://localhost:8080');

const status = await client.workflow(chain([
  { type: 'data.fetch', args: ['https://api.example.com/data'] },
  { type: 'data.transform', args: ['csv'] },
  { type: 'data.load', args: ['warehouse'] },
]));
console.log('Workflow ID:', status.id);
```

### group (parallel)

All jobs run concurrently.

```js
import { group } from './pkg/ojs_wasm_sdk.js';

const status = await client.workflow(group([
  { type: 'export.csv', args: ['rpt_456'] },
  { type: 'export.pdf', args: ['rpt_456'] },
  { type: 'export.xlsx', args: ['rpt_456'] },
]));
```

### batch (parallel with callbacks)

Like a group, but fires callback jobs based on collective outcome.

```js
import { batch } from './pkg/ojs_wasm_sdk.js';

const status = await client.workflow(batch(
  [
    { type: 'email.send', args: ['user1@example.com'] },
    { type: 'email.send', args: ['user2@example.com'] },
  ],
  {
    on_complete: { type: 'batch.report', args: [] },
    on_failure: { type: 'batch.alert', args: [] },
  },
));
```

### Get workflow status

```js
const wfStatus = await client.get_workflow(status.id);
console.log('State:', wfStatus.state);
console.log('Progress:', wfStatus.metadata?.completed_count, '/', wfStatus.metadata?.job_count);
```

---

## Service Worker Integration

`ServiceWorkerClient` uses the global `fetch()` (no `window` dependency) and adds
Background Sync and push notification support.

### Basic usage

```js
// sw.js -- inside your Service Worker
import init, { ServiceWorkerClient } from '@openjobspec/wasm';

await init();
const client = new ServiceWorkerClient('https://api.example.com');

const job = await client.enqueue('email.send', ['user@example.com']);
```

### Background Sync (offline enqueueing)

Register a job for deferred enqueue when the device is offline. The browser
will replay it once connectivity is restored.

```js
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

### Push notifications for job completion

```js
// sw.js
self.addEventListener('push', (event) => {
  const data = event.data.json();
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
| `enqueue_with_options(type, args, options)` | Enqueue with options |
| `enqueue_batch(jobs)` | Batch enqueue multiple jobs |
| `get_job(id)` | Get job details by ID |
| `cancel_job(id)` | Cancel a job by ID |
| `workflow(definition)` | Create and start a workflow |
| `get_workflow(id)` | Get workflow status |
| `health()` | Server health check |
| `register_sync(type, args)` | Register a job for Background Sync |
| `process_sync(tag)` | Process a pending sync tag |
| `notify_job_completed(id, type, state)` | Show a push notification |

---

## Edge Runtime Usage

Edge clients use the global `fetch()` and work in any environment without a
`window` object.

### Generic edge client

Works in any runtime with a global `fetch` (Service Workers, Bun, Node 18+, etc.):

```js
import init, { EdgeClient } from '@openjobspec/wasm';

await init();
const client = new EdgeClient('https://ojs.example.com');
const job = await client.enqueue('email.send', ['user@example.com']);
```

With authentication:

```js
const client = EdgeClient.with_auth('https://ojs.example.com', 'my-api-key');
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

  const client = new VercelEdgeClient('https://ojs.example.com');

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
| `<Client>.with_auth(url, key)` | Create with Bearer token auth |
| `enqueue(type, args)` | Enqueue a single job |
| `enqueue_with_options(type, args, options)` | Enqueue with options |
| `enqueue_batch(jobs)` | Batch enqueue multiple jobs |
| `get_job(id)` | Get job details by ID |
| `cancel_job(id)` | Cancel a job by ID |
| `workflow(definition)` | Create and start a workflow |
| `get_workflow(id)` | Get workflow status |
| `health()` | Server health check |

Runtime-specific extras:

| Client | Method | Description |
|--------|--------|-------------|
| `CloudflareClient` | `enqueue_with_wait_until(ctx, type, args)` | Fire-and-forget via `ctx.waitUntil` |
| `DenoClient` | `DenoClient.from_env()` | Create client from `OJS_URL` env var |
| `VercelEdgeClient` | `VercelEdgeClient.from_env()` | Create client from `OJS_URL` env var |
| `VercelEdgeClient` | `enqueue_with_wait_until(fn, type, args)` | Fire-and-forget via Vercel `waitUntil` |

---

## EnqueueOptions Reference

| Field | Type | Description |
|-------|------|-------------|
| `queue` | `string` | Target queue name (default: `"default"`) |
| `priority` | `number` | Job priority (higher runs first) |
| `timeout_ms` | `number` | Max execution time in milliseconds |
| `delay_until` | `string` | RFC 3339 timestamp to delay job until |
| `tags` | `string[]` | Arbitrary tags for filtering/grouping |

---

## Architecture

```
+---------------------------------------------------+
|  ojs-wasm-sdk                                     |
|                                                   |
|  +---------------------------------------------+ |
|  |  Rust core (wasm32-unknown-unknown)          | |
|  |                                              | |
|  |  OJSClient            (window.fetch)         | |
|  |  ServiceWorkerClient  (global fetch)         | |
|  |  EdgeClient           (global fetch)         | |
|  |  CloudflareClient     (+ ctx.waitUntil)      | |
|  |  DenoClient           (+ Deno.env)           | |
|  |  VercelEdgeClient     (+ process.env)        | |
|  |                                              | |
|  |  Workflow builders: chain, group, batch       | |
|  +---------------------------------------------+ |
|                                                   |
|  +---------------------------------------------+ |
|  |  JavaScript bindings (wasm-bindgen)          | |
|  +---------------------------------------------+ |
|                                                   |
|  +---------------------------------------------+ |
|  |  TypeScript type definitions (.d.ts)         | |
|  +---------------------------------------------+ |
+---------------------------------------------------+
```

---

## Bundle Size and Performance

| Metric | Value |
|--------|-------|
| `.wasm` binary (release, LTO) | ~120 KB |
| Gzipped `.wasm` | ~45 KB |
| JS glue code | ~8 KB |
| `init()` time (cold) | ~5-15 ms |
| `enqueue()` overhead vs raw fetch | < 1 ms |
| JSON serialization throughput | >100K ops/sec |

The WASM binary is compiled with `opt-level = "s"` (optimize for size) and
link-time optimization (LTO) enabled. The release profile produces the smallest
binary suitable for production use.

To check bundle size locally:

```bash
make build-release
gzip -c pkg/ojs_wasm_sdk_bg.wasm | wc -c
```

---

## Limitations and Known Issues

- **Client-only.** This SDK is a job *producer*. There is no worker/consumer
  functionality because browsers and edge runtimes cannot run long-lived polling
  loops. Use the [Go](../ojs-go-sdk/), [Rust](../ojs-rust-sdk/),
  [JS](../ojs-js-sdk/), or [Python](../ojs-python-sdk/) SDKs for workers.
- **No streaming.** HTTP responses are buffered entirely before parsing.
- **Single-threaded.** WASM runs on the main thread (or a worker thread). There
  is no internal concurrency; each operation awaits sequentially.
- **CORS required.** When calling an OJS server from a browser, the server must
  include the appropriate `Access-Control-Allow-Origin` headers.
- **`init()` must be called first.** The WASM module must be initialized before
  using any client. Subsequent `init()` calls are no-ops.
- **No WebSocket support.** Real-time job status updates are not supported;
  use polling with `get_job()` instead.
- **Cold start on edge runtimes.** The first invocation incurs a ~5-15ms
  initialization cost for WASM instantiation.

---

## Build Commands

```bash
make build          # Build for browser (web target)
make build-release  # Optimized release build
make build-bundler  # Build for bundler (webpack/Vite)
make test           # Run tests in headless Chrome
make check          # Fast compilation check
make lint           # Clippy lint
make clean          # Remove build artifacts
```

---

## Development Guide

### Prerequisites

- [Rust 1.75+](https://rustup.rs/) with `wasm32-unknown-unknown` target
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- Chrome or Chromium (for headless tests)

```bash
# Install the WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

### Project Structure

```
ojs-wasm-sdk/
├── src/
│   ├── lib.rs              # OJSClient (browser) and public API
│   ├── types.rs            # Shared types (Job, EnqueueOptions, etc.)
│   ├── transport.rs        # window.fetch HTTP transport
│   ├── edge.rs             # Edge runtime clients (CF, Deno, Vercel)
│   ├── service_worker.rs   # Service Worker client + Background Sync
│   ├── workflow.rs         # Workflow builders (chain, group, batch)
│   ├── middleware.rs        # Request middleware chain
│   ├── retry.rs            # Retry policy configuration
│   ├── queue.rs            # Queue management operations
│   └── error.rs            # Error types
├── tests/
│   └── web.rs              # wasm-bindgen-test browser tests
├── benches/
│   └── serialization.rs    # Serialization performance benchmarks
├── examples/
│   ├── browser/            # Browser HTML demo
│   ├── cloudflare-worker/  # Cloudflare Worker example
│   └── deno/               # Deno Deploy example
├── Cargo.toml
├── package.json
└── ojs-wasm-sdk.d.ts       # Hand-written TypeScript definitions
```

### Running Tests

```bash
# Run all tests in headless Chrome
make test

# Or with wasm-pack directly
wasm-pack test --headless --chrome

# Run native benchmarks
cargo bench
```

### Feature Flags

| Feature | Description |
|---------|-------------|
| `edge_cloudflare` | Enables Cloudflare Workers–specific optimizations |
| `edge_deno` | Enables Deno-specific optimizations |
| `edge_vercel` | Enables Vercel Edge–specific optimizations |
| `edge_all` | Enables all edge runtime features |

### Making Changes

1. Edit Rust sources in `src/`
2. Run `make check` for fast feedback
3. Run `make test` to verify in a real browser environment
4. Run `make build-release` and check bundle size stays under 50KB gzipped
5. Update `ojs-wasm-sdk.d.ts` if the public API changed

---

## Examples

- [`examples/browser/index.html`](examples/browser/index.html) — Interactive browser demo with enqueue, lookup, cancel, and workflow creation
- [`examples/cloudflare-worker/worker.js`](examples/cloudflare-worker/worker.js) — Cloudflare Worker with `waitUntil` integration
- [`examples/deno/main.ts`](examples/deno/main.ts) — Deno Deploy HTTP server

Legacy flat examples (kept for compatibility):

- [`examples/browser.html`](examples/browser.html) — Browser demo (flat)
- [`examples/cloudflare-worker.js`](examples/cloudflare-worker.js) — Cloudflare Worker (flat)
- [`examples/deno-deploy.ts`](examples/deno-deploy.ts) — Deno Deploy (flat)
- [`examples/vercel-edge.ts`](examples/vercel-edge.ts) — Vercel Edge Function

---

## Contributing

1. Join the [discussion](https://github.com/openjobspec/spec/discussions)
2. Review the [OJS Rust SDK](../ojs-rust-sdk/) which serves as the WASM core
3. Check the [Roadmap](../ROADMAP.md) for timeline updates

## License

[Apache License 2.0](../LICENSE)
