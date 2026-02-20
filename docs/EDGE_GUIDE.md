# Edge Runtime Guide

This guide covers deploying the OJS WASM SDK on edge runtimes: Cloudflare Workers, Deno Deploy, and Vercel Edge Functions.

## Why WASM on the Edge?

The OJS WASM SDK compiles the Rust OJS client to WebAssembly, running natively in V8 isolates on edge platforms. Benefits:

- **Sub-millisecond cold starts** — WASM modules load faster than JavaScript bundles
- **Small footprint** — ~50 KB gzipped, no dependencies
- **Consistent behavior** — Same Rust logic across all platforms
- **Global distribution** — Run at 300+ edge locations

## Cloudflare Workers

### Setup

```bash
npm create cloudflare@latest my-ojs-worker
cd my-ojs-worker
npm install @openjobspec/wasm
```

### Worker Code

```typescript
// src/worker.ts
import init, { CloudflareClient, chain } from '@openjobspec/wasm';

let initialized = false;

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    if (!initialized) {
      await init();
      initialized = true;
    }

    const client = new CloudflareClient(env.OJS_URL);

    const url = new URL(request.url);

    if (url.pathname === '/enqueue' && request.method === 'POST') {
      const body = await request.json() as { type: string; args: unknown[] };
      const job = await client.enqueue(body.type, body.args);
      return Response.json(job, { status: 201 });
    }

    // Fire-and-forget: response returns immediately, enqueue happens in background
    if (url.pathname === '/enqueue-bg' && request.method === 'POST') {
      const body = await request.json() as { type: string; args: unknown[] };
      client.enqueue_with_wait_until(ctx, body.type, body.args);
      return new Response('accepted', { status: 202 });
    }

    if (url.pathname === '/health') {
      const health = await client.health();
      return Response.json(health);
    }

    return new Response('OJS Worker', { status: 200 });
  },
};

interface Env {
  OJS_URL: string;
}
```

### Wrangler Configuration

```toml
# wrangler.toml
name = "ojs-worker"
main = "src/worker.ts"
compatibility_date = "2024-01-01"

[vars]
OJS_URL = "https://ojs.example.com"
```

### Key Feature: `waitUntil`

Cloudflare Workers can return a response immediately while continuing work in the background using `ctx.waitUntil()`. The `CloudflareClient` wraps this:

```typescript
// Response returns immediately with 202
// The enqueue HTTP call runs in the background
client.enqueue_with_wait_until(ctx, 'analytics.track', [userId, event]);
```

### Deploy

```bash
npx wrangler deploy
```

## Deno Deploy

### Setup

No build step required. Import directly from esm.sh:

```typescript
import init, { DenoClient } from 'https://esm.sh/@openjobspec/wasm';
```

### Server Code

```typescript
// main.ts
import init, { DenoClient, chain, group } from 'https://esm.sh/@openjobspec/wasm';

await init();
const client = DenoClient.from_env(); // reads OJS_URL from environment

Deno.serve({ port: 3000 }, async (req: Request) => {
  const url = new URL(req.url);

  if (url.pathname === '/enqueue' && req.method === 'POST') {
    const body = await req.json();
    const job = await client.enqueue(body.type, body.args || []);
    return Response.json(job, { status: 201 });
  }

  if (url.pathname.startsWith('/job/') && req.method === 'GET') {
    const id = url.pathname.split('/job/')[1];
    const job = await client.get_job(id);
    return Response.json(job);
  }

  if (url.pathname === '/health') {
    return Response.json(await client.health());
  }

  return new Response('OJS on Deno', { status: 200 });
});
```

### Deploy

```bash
# Local
OJS_URL=http://localhost:8080 deno run --allow-net --allow-env main.ts

# Deno Deploy
deployctl deploy --project=my-ojs main.ts
```

## Vercel Edge Functions

### Setup

```bash
npm install @openjobspec/wasm @vercel/functions
```

### API Route (Next.js App Router)

```typescript
// app/api/enqueue/route.ts
import init, { VercelEdgeClient } from '@openjobspec/wasm';
import { waitUntil } from '@vercel/functions';

export const runtime = 'edge';

let client: InstanceType<typeof VercelEdgeClient>;

async function getClient() {
  if (!client) {
    await init();
    client = VercelEdgeClient.from_env();
  }
  return client;
}

export async function POST(req: Request) {
  const c = await getClient();
  const body = await req.json();

  const job = await c.enqueue(body.type, body.args || []);

  // Fire-and-forget analytics in background
  c.enqueue_with_wait_until(waitUntil, 'analytics.api_call', [req.url]);

  return Response.json(job, { status: 201 });
}

export async function GET(req: Request) {
  const c = await getClient();
  const url = new URL(req.url);
  const jobId = url.searchParams.get('id');

  if (!jobId) {
    return Response.json({ error: 'id parameter required' }, { status: 400 });
  }

  const job = await c.get_job(jobId);
  return Response.json(job);
}
```

### Key Feature: `waitUntil`

Vercel Edge Functions support `waitUntil` for fire-and-forget operations:

```typescript
import { waitUntil } from '@vercel/functions';

// Returns response immediately, enqueue runs in background
c.enqueue_with_wait_until(waitUntil, 'analytics.pageview', [path]);
```

### Deploy

```bash
vercel deploy --prod
```

## Authentication on Edge

All edge clients support Bearer token authentication:

```typescript
// Static API key
const client = CloudflareClient.with_auth('https://ojs.example.com', 'your-api-key');

// From environment (Deno, Vercel)
const client = DenoClient.from_env();      // reads OJS_URL and OJS_API_KEY
const client = VercelEdgeClient.from_env(); // reads OJS_URL and OJS_API_KEY
```

## Middleware

Add middleware to intercept and modify requests before they are sent:

```typescript
import { MiddlewareChain, create_request } from '@openjobspec/wasm';

const mw = new MiddlewareChain();

// Add a request ID header to every request
mw.add('request-id', (req) => {
  req.headers['X-Request-ID'] = crypto.randomUUID();
  return req;
});

// Add timing
mw.add('timing', (req) => {
  req.headers['X-Request-Start'] = Date.now().toString();
  return req;
});

console.log('Active middleware:', mw.list()); // ['request-id', 'timing']
```

## Performance Tips

1. **Initialize once** — Call `init()` at startup, not per-request
2. **Reuse clients** — Create one client instance and reuse it
3. **Use `waitUntil`** — For non-critical enqueues, use fire-and-forget to reduce latency
4. **Batch enqueue** — Use `enqueue_batch()` to send multiple jobs in one HTTP call:

```typescript
const jobs = await client.enqueue_batch([
  { type: 'email.send', args: ['a@example.com', 'Hello'] },
  { type: 'email.send', args: ['b@example.com', 'Hello'] },
  { type: 'email.send', args: ['c@example.com', 'Hello'] },
]);
```
