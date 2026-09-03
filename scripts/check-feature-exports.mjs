import { spawnSync } from 'node:child_process';
import { mkdir, readFile, rm } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const outputRoot = join(root, '.feature-matrix');

const establishedDefaultAbi = [
  'CloudflareClient',
  'D1DatabaseRef',
  'DenoClient',
  'DurableContext',
  'EdgeClient',
  'EncryptionCodec',
  'FakeStore',
  'KVNamespaceRef',
  'MiddlewareChain',
  'OJSClient',
  'RetryPolicy',
  'SSESubscription',
  'SchemaValidator',
  'ServiceWorkerClient',
  'VercelEdgeClient',
  'batch',
  'chain',
  'create_request',
  'decrypt_args',
  'default',
  'encrypt_args',
  'group',
  'initSync',
  'subscribe_job',
  'subscribe_queue',
];

const groups = {
  cloudflare: ['CloudflareClient', 'D1DatabaseRef', 'KVNamespaceRef'],
  deno: ['DenoClient'],
  encryption: ['EncryptionCodec', 'decrypt_args', 'encrypt_args'],
  schema: ['SchemaValidator'],
  serviceWorker: ['ServiceWorkerClient', 'background_sync_tag_prefix'],
  vercel: ['VercelEdgeClient'],
};

const variants = [
  { name: 'core', features: [], required: [], forbidden: Object.values(groups).flat() },
  {
    name: 'service-worker',
    features: ['service_worker'],
    required: groups.serviceWorker,
    forbidden: [...groups.cloudflare, ...groups.deno, ...groups.encryption, ...groups.schema, ...groups.vercel],
  },
  {
    name: 'encryption',
    features: ['encryption'],
    required: groups.encryption,
    forbidden: [...groups.cloudflare, ...groups.deno, ...groups.schema, ...groups.serviceWorker, ...groups.vercel],
  },
  {
    name: 'schema',
    features: ['schema'],
    required: groups.schema,
    forbidden: [...groups.cloudflare, ...groups.deno, ...groups.encryption, ...groups.serviceWorker, ...groups.vercel],
  },
  {
    name: 'edge-cloudflare',
    features: ['edge_cloudflare'],
    required: groups.cloudflare,
    forbidden: [...groups.deno, ...groups.encryption, ...groups.schema, ...groups.serviceWorker, ...groups.vercel],
  },
  {
    name: 'edge-deno',
    features: ['edge_deno'],
    required: groups.deno,
    forbidden: [...groups.cloudflare, ...groups.encryption, ...groups.schema, ...groups.serviceWorker, ...groups.vercel],
  },
  {
    name: 'edge-vercel',
    features: ['edge_vercel'],
    required: groups.vercel,
    forbidden: [...groups.cloudflare, ...groups.deno, ...groups.encryption, ...groups.schema, ...groups.serviceWorker],
  },
  {
    name: 'edge-all',
    features: ['edge_all'],
    required: [...groups.cloudflare, ...groups.deno, ...groups.vercel],
    forbidden: [...groups.encryption, ...groups.schema, ...groups.serviceWorker],
  },
  {
    name: 'default',
    defaultFeatures: true,
    required: [...establishedDefaultAbi, 'background_sync_tag_prefix'],
    forbidden: [],
  },
];

function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', stdio: 'inherit' });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed`);
  }
}

function assertSymbols(name, actual, required, forbidden, declarations) {
  const declared = new Set(
    [...declarations.matchAll(/export (?:class|function) ([A-Za-z_][A-Za-z0-9_]*)/g)].map(
      (match) => match[1],
    ),
  );
  if (declarations.includes('export default')) declared.add('default');

  for (const symbol of required) {
    if (!actual.has(symbol)) throw new Error(`${name}: missing JS export ${symbol}`);
    if (!declared.has(symbol)) throw new Error(`${name}: missing declaration ${symbol}`);
  }
  for (const symbol of forbidden) {
    if (actual.has(symbol)) throw new Error(`${name}: unexpected JS export ${symbol}`);
    if (declared.has(symbol)) throw new Error(`${name}: unexpected declaration ${symbol}`);
  }
}

await rm(outputRoot, { recursive: true, force: true });
await mkdir(outputRoot, { recursive: true });

const sizes = [];
try {
  for (const variant of variants) {
    const output = join('.feature-matrix', variant.name);
    const args = ['build', '--target', 'web', '--release', '--out-dir', output];
    if (!variant.defaultFeatures) {
      args.push('--', '--no-default-features');
      if (variant.features.length > 0) {
        args.push('--features', variant.features.join(','));
      }
    }
    run('wasm-pack', args);

    const directory = join(root, output);
    const module = await import(
      `${pathToFileURL(join(directory, 'ojs_wasm_sdk.js')).href}?variant=${variant.name}`
    );
    const declarations = await readFile(join(directory, 'ojs_wasm_sdk.d.ts'), 'utf8');
    assertSymbols(
      variant.name,
      new Set(Object.keys(module)),
      variant.required,
      variant.forbidden,
      declarations,
    );

    const wasm = await readFile(join(directory, 'ojs_wasm_sdk_bg.wasm'));
    const gzip = spawnSync('gzip', ['-cn', join(directory, 'ojs_wasm_sdk_bg.wasm')], {
      encoding: null,
      maxBuffer: 2 * 1024 * 1024,
    });
    if (gzip.status !== 0) throw new Error(`${variant.name}: gzip failed`);
    sizes.push({
      name: variant.name,
      raw: wasm.byteLength,
      gzip: gzip.stdout.byteLength,
    });
  }

  console.log('\nFeature export matrix passed.');
  console.log('variant,raw_bytes,gzip_bytes');
  for (const size of sizes) {
    console.log(`${size.name},${size.raw},${size.gzip}`);
  }
} finally {
  await rm(outputRoot, { recursive: true, force: true });
}
