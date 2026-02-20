// Vercel Edge Function example for OJS WASM SDK
// Deploy: vercel deploy
//
// Environment variables:
//   OJS_URL=https://your-ojs-server.example.com
//   OJS_API_KEY=your-api-key

import init, { VercelEdgeClient } from '@openjobspec/wasm';
import { waitUntil } from '@vercel/functions';

export const config = { runtime: 'edge' };

let client: InstanceType<typeof VercelEdgeClient>;

async function getClient() {
  if (!client) {
    await init();
    client = VercelEdgeClient.from_env();
    // Or create manually:
    // client = new VercelEdgeClient(process.env.OJS_URL || 'http://localhost:8080');
  }
  return client;
}

export default async function handler(req: Request) {
  const c = await getClient();
  const url = new URL(req.url);

  // Fire-and-forget analytics via Vercel's waitUntil
  c.enqueue_with_wait_until(
    waitUntil,
    'analytics.pageview',
    [url.pathname, req.headers.get('user-agent') || '']
  );

  // Enqueue a background job for each page visit
  const result = await c.enqueue(
    'page.generate',
    [url.pathname]
  );

  return new Response(JSON.stringify({
    message: 'Job enqueued from Vercel Edge!',
    job: result,
  }), {
    headers: { 'content-type': 'application/json' },
  });
}
