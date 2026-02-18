# OJS WASM SDK

> ⚠️ **Status: Planning** — This SDK is in the design phase and not yet implemented.

## Vision

A browser-native OJS client compiled to WebAssembly, enabling:

- **Edge/Serverless**: Run OJS workers in CloudFlare Workers, Vercel Edge Functions, Deno Deploy
- **Browser**: Enqueue and monitor jobs directly from web applications
- **Isomorphic**: Share job type definitions between server and client
- **Zero-install playground**: Power the [OJS Playground](https://playground.openjobspec.org) with a fully local OJS runtime

## Planned Architecture

```
┌─────────────────────────────────┐
│  ojs-wasm-sdk                   │
│  ┌───────────────────────────┐  │
│  │  Rust core (compiled to   │  │
│  │  wasm32-unknown-unknown)  │  │
│  └───────────────────────────┘  │
│  ┌───────────────────────────┐  │
│  │  JavaScript bindings      │  │
│  │  (wasm-bindgen / wasm-pack)│  │
│  └───────────────────────────┘  │
│  ┌───────────────────────────┐  │
│  │  TypeScript type defs     │  │
│  └───────────────────────────┘  │
└─────────────────────────────────┘
```

## Target API

```typescript
import { OJSClient } from '@openjobspec/wasm';

const client = new OJSClient({ url: 'http://localhost:8080' });

// Enqueue from the browser
await client.enqueue('email.send', ['user@example.com', 'Hello!']);

// Monitor job status
const job = await client.getJob(jobId);
```

## Contributing

If you're interested in contributing to the WASM SDK, please:

1. Join the [discussion](https://github.com/openjobspec/spec/discussions)
2. Review the [OJS Rust SDK](../ojs-rust-sdk/) which will serve as the WASM core
3. Check the [Roadmap](../ROADMAP.md) for timeline updates

## License

[Apache License 2.0](../LICENSE)
