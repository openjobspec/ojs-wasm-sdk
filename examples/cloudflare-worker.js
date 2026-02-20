/**
 * Cloudflare Worker example using the OJS WASM SDK.
 *
 * Setup:
 *   1. npm create cloudflare@latest my-ojs-worker
 *   2. npm install @openjobspec/wasm
 *   3. Copy this file to src/index.js
 *   4. npx wrangler dev
 */

import init, { CloudflareClient, chain } from '@openjobspec/wasm';

let initialized = false;

export default {
  async fetch(request, env, ctx) {
    // Initialize WASM once (subsequent calls are no-ops)
    if (!initialized) {
      await init();
      initialized = true;
    }

    const url = new URL(request.url);
    const client = new CloudflareClient(env.OJS_URL || 'http://localhost:8080');

    // POST /enqueue — enqueue a job
    if (url.pathname === '/enqueue' && request.method === 'POST') {
      const body = await request.json();
      const job = await client.enqueue(body.type, body.args || []);
      return Response.json(job, { status: 201 });
    }

    // POST /enqueue-background — fire-and-forget enqueue via waitUntil
    if (url.pathname === '/enqueue-background' && request.method === 'POST') {
      const body = await request.json();
      client.enqueue_with_wait_until(ctx, body.type, body.args || []);
      return new Response('accepted', { status: 202 });
    }

    // GET /job/:id — get job status
    if (url.pathname.startsWith('/job/') && request.method === 'GET') {
      const id = url.pathname.split('/job/')[1];
      const job = await client.get_job(id);
      return Response.json(job);
    }

    // POST /workflow — create a chain workflow
    if (url.pathname === '/workflow' && request.method === 'POST') {
      const body = await request.json();
      const definition = chain(body.steps);
      const status = await client.workflow(definition);
      return Response.json(status, { status: 201 });
    }

    // GET /health — proxy health check
    if (url.pathname === '/health') {
      const health = await client.health();
      return Response.json(health);
    }

    return new Response('OJS Cloudflare Worker', { status: 200 });
  },
};
