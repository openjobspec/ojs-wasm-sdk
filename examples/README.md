# OJS WASM SDK Examples

Working examples demonstrating the OJS WASM SDK across different JavaScript runtimes.

## Examples

| Example | Runtime | Description |
|---------|---------|-------------|
| [browser/](./browser/) | Browser | Interactive web UI for enqueuing and managing jobs |
| [react-app/](./react-app/) | React | React integration with custom `useOJS` hook |
| [next-js/](./next-js/) | Next.js | Next.js App Router with Edge API routes |
| [cloudflare-worker/](./cloudflare-worker/) | Cloudflare Workers | Worker with `waitUntil` fire-and-forget |
| [deno/](./deno/) | Deno | Deno HTTP server with environment config |
| [vercel-edge.ts](./vercel-edge.ts) | Vercel Edge | Vercel Edge Function with `waitUntil` |
| [deno-deploy.ts](./deno-deploy.ts) | Deno Deploy | Single-file Deno Deploy server |

## Prerequisites

All examples require an OJS-compatible server running. Start one with:

```bash
# Using Docker (quickest)
cd ../.. && docker compose -f docker-compose.quickstart.yml up -d

# Or run ojs-backend-redis locally
cd ../../ojs-backend-redis && make run
```

## Running Examples

### Browser

```bash
# Build the WASM package first
cd .. && make build

# Serve the examples directory
npx serve .

# Open http://localhost:3000/examples/browser/
```

### React App

```bash
cd react-app
npm install
npm run dev
# Open http://localhost:5173
```

### Next.js

```bash
cd next-js
npm install
npm run dev
# Open http://localhost:3000
```

### Cloudflare Worker

```bash
cd cloudflare-worker
npm install
npx wrangler dev
```

### Deno

```bash
cd deno
OJS_URL=http://localhost:8080 deno run --allow-net --allow-env main.ts
```
