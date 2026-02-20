/**
 * Deno Deploy example using the OJS WASM SDK.
 *
 * Usage:
 *   OJS_URL=http://localhost:8080 deno run --allow-net --allow-env deno-deploy.ts
 */

import init, { DenoClient, chain, group } from '@openjobspec/wasm';

await init();

// Read OJS_URL from environment, or fall back to localhost
const client = (() => {
  try {
    return DenoClient.from_env();
  } catch {
    return new DenoClient('http://localhost:8080');
  }
})();

Deno.serve({ port: 3000 }, async (req: Request) => {
  const url = new URL(req.url);

  // POST /enqueue
  if (url.pathname === '/enqueue' && req.method === 'POST') {
    const body = await req.json();
    const job = await client.enqueue(body.type, body.args || []);
    return Response.json(job, { status: 201 });
  }

  // POST /enqueue-with-options
  if (url.pathname === '/enqueue-with-options' && req.method === 'POST') {
    const body = await req.json();
    const job = await client.enqueue_with_options(
      body.type,
      body.args || [],
      body.options || {},
    );
    return Response.json(job, { status: 201 });
  }

  // GET /job/:id
  if (url.pathname.startsWith('/job/') && req.method === 'GET') {
    const id = url.pathname.split('/job/')[1];
    const job = await client.get_job(id);
    return Response.json(job);
  }

  // DELETE /job/:id
  if (url.pathname.startsWith('/job/') && req.method === 'DELETE') {
    const id = url.pathname.split('/job/')[1];
    const job = await client.cancel_job(id);
    return Response.json(job);
  }

  // POST /workflow/chain
  if (url.pathname === '/workflow/chain' && req.method === 'POST') {
    const body = await req.json();
    const definition = chain(body.steps);
    const status = await client.workflow(definition);
    return Response.json(status, { status: 201 });
  }

  // POST /workflow/group
  if (url.pathname === '/workflow/group' && req.method === 'POST') {
    const body = await req.json();
    const definition = group(body.jobs);
    const status = await client.workflow(definition);
    return Response.json(status, { status: 201 });
  }

  // GET /health
  if (url.pathname === '/health') {
    const health = await client.health();
    return Response.json(health);
  }

  return new Response('OJS Deno Deploy example', { status: 200 });
});
