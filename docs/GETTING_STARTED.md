# Getting Started with the OJS WASM SDK

This guide walks you through setting up the OJS WASM SDK in different JavaScript environments: browser, edge runtimes, and Deno.

## Prerequisites

- An OJS-compatible server running (e.g., `ojs-backend-redis` at `http://localhost:8080`)
- Node.js 18+ (for npm-based projects)

## Installation

### npm (recommended for bundled projects)

```bash
npm install @openjobspec/wasm
```

### CDN (no build step)

```html
<script type="module">
  import init, { OJSClient } from 'https://unpkg.com/@openjobspec/wasm/ojs_wasm_sdk.js';
  await init();
</script>
```

### Deno

```ts
import init, { DenoClient } from 'https://esm.sh/@openjobspec/wasm';
```

## Step 1: Initialize WASM

The WASM module must be initialized once before creating any client. This loads the `.wasm` binary:

```typescript
import init from '@openjobspec/wasm';

await init();
// Now you can use any client class
```

> **Note:** Calling `init()` multiple times is safe — subsequent calls are no-ops.

## Step 2: Create a Client

Choose the client class that matches your runtime:

| Runtime | Client Class | Import |
|---------|-------------|--------|
| Browser | `OJSClient` | `import { OJSClient } from '@openjobspec/wasm'` |
| Service Worker | `ServiceWorkerClient` | `import { ServiceWorkerClient } from '@openjobspec/wasm'` |
| Cloudflare Workers | `CloudflareClient` | `import { CloudflareClient } from '@openjobspec/wasm'` |
| Vercel Edge | `VercelEdgeClient` | `import { VercelEdgeClient } from '@openjobspec/wasm'` |
| Deno / Deno Deploy | `DenoClient` | `import { DenoClient } from '@openjobspec/wasm'` |
| Node.js 18+ / Bun | `EdgeClient` | `import { EdgeClient } from '@openjobspec/wasm'` |

```typescript
import init, { OJSClient } from '@openjobspec/wasm';

await init();
const client = new OJSClient('http://localhost:8080');
```

## Step 3: Enqueue a Job

```typescript
// Simple enqueue
const job = await client.enqueue('email.send', ['user@example.com', 'Welcome!']);
console.log('Job ID:', job.id);

// Enqueue with options
const priorityJob = await client.enqueue_with_options(
  'report.generate',
  ['monthly', '2024-01'],
  { queue: 'reports', priority: 5 }
);
```

## Step 4: Check Job Status

```typescript
const job = await client.get_job('your-job-id');
console.log('State:', job.state); // 'completed', 'active', etc.
```

## Step 5: Create Workflows

```typescript
import { chain, group, batch } from '@openjobspec/wasm';

// Sequential: step 2 runs after step 1 completes
const sequential = chain([
  { type: 'data.fetch', args: ['https://api.example.com'] },
  { type: 'data.transform', args: ['csv'] },
  { type: 'data.load', args: ['warehouse'] },
]);

// Parallel: all jobs run concurrently
const parallel = group([
  { type: 'image.resize', args: ['photo.jpg', 800, 600] },
  { type: 'image.resize', args: ['photo.jpg', 400, 300] },
  { type: 'image.resize', args: ['photo.jpg', 200, 150] },
]);

const status = await client.workflow(sequential);
console.log('Workflow ID:', status.id);
```

## Step 6: Health Check

```typescript
const health = await client.health();
console.log('Server status:', health.status);
```

## Authentication

For authenticated servers, create a client with an API key:

```typescript
const client = EdgeClient.with_auth('https://ojs.example.com', 'your-api-key');
```

Or for environment-based configuration (Deno, Vercel):

```typescript
// Reads OJS_URL from environment
const client = DenoClient.from_env();
const client = VercelEdgeClient.from_env();
```

## Next Steps

- [Browser Integration Guide](./BROWSER_GUIDE.md) — SPAs, forms, and React hooks
- [Edge Runtime Guide](./EDGE_GUIDE.md) — Cloudflare Workers, Deno Deploy, Vercel Edge
- [API Reference](./API_REFERENCE.md) — Complete API documentation
- [Examples](../examples/README.md) — Working code examples
