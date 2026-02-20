/**
 * OJS WASM SDK — Next.js Edge API Route
 *
 * This API route runs as a Vercel Edge Function and uses the WASM SDK
 * to enqueue jobs and check job status.
 *
 * Endpoints:
 *   POST /api/ojs          — Enqueue a job
 *   GET  /api/ojs?id=...   — Get job status
 *   GET  /api/ojs           — Health check
 *
 * Environment variables:
 *   OJS_URL     — OJS server URL (default: http://localhost:8080)
 *   OJS_API_KEY — Optional API key
 */

import init, { VercelEdgeClient, chain, group } from '@openjobspec/wasm';

export const runtime = 'edge';

let client: InstanceType<typeof VercelEdgeClient>;

async function getClient() {
  if (!client) {
    await init();
    client = new VercelEdgeClient(
      process.env.OJS_URL || 'http://localhost:8080',
    );
  }
  return client;
}

export async function POST(req: Request) {
  const c = await getClient();
  const body = (await req.json()) as {
    type: string;
    args?: unknown[];
    options?: Record<string, unknown>;
    workflow?: { type: string; steps?: unknown[]; jobs?: unknown[] };
  };

  // Workflow creation
  if (body.workflow) {
    const definition =
      body.workflow.type === 'chain'
        ? chain(body.workflow.steps as any[])
        : group(body.workflow.jobs as any[]);
    const status = await c.workflow(definition);
    return Response.json(status, { status: 201 });
  }

  // Single job enqueue
  const job = body.options
    ? await c.enqueue_with_options(body.type, body.args || [], body.options as any)
    : await c.enqueue(body.type, body.args || []);

  return Response.json(job, { status: 201 });
}

export async function GET(req: Request) {
  const c = await getClient();
  const url = new URL(req.url);
  const jobId = url.searchParams.get('id');

  if (jobId) {
    const job = await c.get_job(jobId);
    return Response.json(job);
  }

  // Default: health check
  const health = await c.health();
  return Response.json(health);
}

export async function DELETE(req: Request) {
  const c = await getClient();
  const url = new URL(req.url);
  const jobId = url.searchParams.get('id');

  if (!jobId) {
    return Response.json({ error: 'id parameter required' }, { status: 400 });
  }

  const job = await c.cancel_job(jobId);
  return Response.json(job);
}
