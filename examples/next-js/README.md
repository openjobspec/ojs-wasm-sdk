# OJS WASM SDK — Next.js Example

A Next.js application demonstrating the OJS WASM SDK with Edge API routes.

## Quick Start

```bash
npm install
npm run dev
# Open http://localhost:3000
```

## Environment Variables

Create a `.env.local` file:

```
OJS_URL=http://localhost:8080
OJS_API_KEY=your-api-key  # optional
```

## API Endpoints

### `POST /api/ojs` — Enqueue a job

```bash
curl -X POST http://localhost:3000/api/ojs \
  -H 'Content-Type: application/json' \
  -d '{"type": "email.send", "args": ["user@example.com", "Hello"]}'
```

### `GET /api/ojs?id=<job-id>` — Get job status

```bash
curl http://localhost:3000/api/ojs?id=your-job-id
```

### `GET /api/ojs` — Health check

```bash
curl http://localhost:3000/api/ojs
```

### `DELETE /api/ojs?id=<job-id>` — Cancel a job

```bash
curl -X DELETE http://localhost:3000/api/ojs?id=your-job-id
```

### `POST /api/ojs` — Create a workflow

```bash
curl -X POST http://localhost:3000/api/ojs \
  -H 'Content-Type: application/json' \
  -d '{
    "workflow": {
      "type": "chain",
      "steps": [
        {"type": "data.fetch", "args": ["source"]},
        {"type": "data.transform", "args": ["csv"]}
      ]
    }
  }'
```

## Architecture

The API route runs as a **Vercel Edge Function** — the WASM module executes in V8 isolates with sub-millisecond cold starts. The route can be deployed to Vercel or run locally with `next dev`.
