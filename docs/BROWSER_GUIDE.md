# Browser Integration Guide

This guide covers using the OJS WASM SDK in browser environments: vanilla JavaScript, React, and other frontend frameworks.

## Basic Setup

### With a Bundler (Vite, webpack, Rollup)

```bash
npm install @openjobspec/wasm
```

```typescript
import init, { OJSClient } from '@openjobspec/wasm';

async function setupOJS() {
  await init();
  return new OJSClient('http://localhost:8080');
}
```

### Without a Bundler (CDN)

```html
<script type="module">
  import init, { OJSClient } from 'https://unpkg.com/@openjobspec/wasm/ojs_wasm_sdk.js';

  await init();
  const client = new OJSClient('http://localhost:8080');

  const job = await client.enqueue('email.send', ['user@example.com', 'Hello']);
  console.log('Enqueued:', job.id);
</script>
```

## Singleton Pattern

Initialize WASM once and reuse the client across your app:

```typescript
// ojs.ts
import init, { OJSClient } from '@openjobspec/wasm';

let client: OJSClient | null = null;

export async function getClient(): Promise<OJSClient> {
  if (!client) {
    await init();
    client = new OJSClient(import.meta.env.VITE_OJS_URL || 'http://localhost:8080');
  }
  return client;
}
```

## React Integration

### Custom Hook

```tsx
// hooks/useOJS.ts
import { useState, useEffect, useCallback, useRef } from 'react';
import init, { OJSClient } from '@openjobspec/wasm';
import type { Job, EnqueueOptions } from '@openjobspec/wasm';

export function useOJS(serverUrl: string) {
  const clientRef = useRef<OJSClient | null>(null);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let cancelled = false;
    init()
      .then(() => {
        if (!cancelled) {
          clientRef.current = new OJSClient(serverUrl);
          setReady(true);
        }
      })
      .catch((err) => {
        if (!cancelled) setError(err);
      });
    return () => { cancelled = true; };
  }, [serverUrl]);

  const enqueue = useCallback(
    async (type: string, args: unknown[], options?: EnqueueOptions) => {
      if (!clientRef.current) throw new Error('OJS not initialized');
      return options
        ? clientRef.current.enqueue_with_options(type, args, options)
        : clientRef.current.enqueue(type, args);
    },
    [],
  );

  const getJob = useCallback(async (id: string) => {
    if (!clientRef.current) throw new Error('OJS not initialized');
    return clientRef.current.get_job(id);
  }, []);

  const cancelJob = useCallback(async (id: string) => {
    if (!clientRef.current) throw new Error('OJS not initialized');
    return clientRef.current.cancel_job(id);
  }, []);

  return { ready, error, enqueue, getJob, cancelJob };
}
```

### Usage in Components

```tsx
import { useOJS } from './hooks/useOJS';

function ContactForm() {
  const { ready, enqueue } = useOJS('http://localhost:8080');

  async function handleSubmit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const form = new FormData(e.currentTarget);

    const job = await enqueue('email.send', [
      form.get('email'),
      form.get('subject'),
      form.get('message'),
    ]);

    alert(`Email queued! Job ID: ${job.id}`);
  }

  if (!ready) return <p>Loading OJS...</p>;

  return (
    <form onSubmit={handleSubmit}>
      <input name="email" type="email" placeholder="Recipient" required />
      <input name="subject" type="text" placeholder="Subject" required />
      <textarea name="message" placeholder="Message" required />
      <button type="submit">Send Email</button>
    </form>
  );
}
```

### Job Status Polling

```tsx
import { useState, useEffect } from 'react';
import { useOJS } from './hooks/useOJS';
import type { Job } from '@openjobspec/wasm';

function JobTracker({ jobId }: { jobId: string }) {
  const { ready, getJob } = useOJS('http://localhost:8080');
  const [job, setJob] = useState<Job | null>(null);

  useEffect(() => {
    if (!ready || !jobId) return;
    const interval = setInterval(async () => {
      const j = await getJob(jobId);
      setJob(j);
      if (j.state === 'completed' || j.state === 'discarded' || j.state === 'cancelled') {
        clearInterval(interval);
      }
    }, 2000);
    return () => clearInterval(interval);
  }, [ready, jobId, getJob]);

  if (!job) return <p>Loading...</p>;

  return (
    <div>
      <p>Job: {job.id}</p>
      <p>State: <strong>{job.state}</strong></p>
      <p>Type: {job.type}</p>
      <p>Attempt: {job.attempt}</p>
    </div>
  );
}
```

## Service Worker (Offline Support)

The `ServiceWorkerClient` enables offline job enqueueing via the Background Sync API:

```typescript
// sw.ts (service worker)
import init, { ServiceWorkerClient } from '@openjobspec/wasm';

let client: ServiceWorkerClient;

self.addEventListener('install', () => {
  // @ts-ignore
  self.skipWaiting();
});

self.addEventListener('activate', async () => {
  await init();
  client = new ServiceWorkerClient('https://ojs.example.com');
});

// Queue jobs for Background Sync when offline
self.addEventListener('message', async (event) => {
  if (event.data.type === 'ENQUEUE_JOB') {
    const tag = await client.register_sync(event.data.jobType, event.data.args);
    event.ports[0].postMessage({ tag });
  }
});

// Process synced jobs when back online
self.addEventListener('sync', async (event: any) => {
  if (event.tag.startsWith('ojs-sync-')) {
    event.waitUntil(client.process_sync(event.tag));
  }
});
```

## CORS Configuration

When the browser client and OJS server are on different origins, configure CORS on the server:

```
Access-Control-Allow-Origin: https://your-app.com
Access-Control-Allow-Methods: GET, POST, DELETE
Access-Control-Allow-Headers: Content-Type, Authorization
```

Most OJS backends include CORS support. Check your backend's documentation for configuration options.

## Bundle Size

The WASM binary is typically **under 50 KB gzipped**. For optimal loading:

1. Ensure your server sends `Content-Type: application/wasm` for `.wasm` files
2. Enable gzip/brotli compression on your CDN
3. Use `init()` lazily (e.g., on first user interaction, not on page load)

```typescript
// Lazy initialization on first use
let initPromise: Promise<void> | null = null;
function ensureInit() {
  if (!initPromise) initPromise = init();
  return initPromise;
}
```
