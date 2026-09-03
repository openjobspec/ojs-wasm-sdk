import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { access, mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { dirname, extname, join, resolve, sep } from 'node:path';
import { homedir } from 'node:os';
import { fileURLToPath } from 'node:url';
import WebSocket from 'ws';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const scratch = join(root, `.npm-package-smoke-${process.pid}-${Date.now()}`);
const consumer = join(scratch, 'consumer');

function run(command, args, cwd, options = {}) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: 'utf8',
    stdio: options.capture ? 'pipe' : 'inherit',
    ...options,
  });
  if (result.status !== 0) {
    const details = options.capture
      ? `\n${result.stdout ?? ''}\n${result.stderr ?? ''}`
      : '';
    throw new Error(`${command} ${args.join(' ')} failed${details}`);
  }
  return result.stdout ?? '';
}

async function findChrome() {
  const candidates = [process.env.CHROME_BIN];
  if (process.platform === 'darwin') {
    try {
      const installations = await readdir(join(homedir(), '.agent-browser', 'browsers'));
      for (const installation of installations.sort().reverse()) {
        candidates.push(
          join(
            homedir(),
            '.agent-browser',
            'browsers',
            installation,
            'Google Chrome for Testing.app',
            'Contents',
            'MacOS',
            'Google Chrome for Testing',
          ),
        );
      }
    } catch {
      // Optional agent-browser installation is not present.
    }
  }
  candidates.push(
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    '/Applications/Chromium.app/Contents/MacOS/Chromium',
    'google-chrome-stable',
    'google-chrome',
    'chromium',
    'chromium-browser',
  );

  for (const candidate of candidates.filter(Boolean)) {
    const result = spawnSync(candidate, ['--version'], {
      encoding: 'utf8',
      stdio: 'pipe',
    });
    if (result.status === 0) return candidate;
  }
  throw new Error('Chrome/Chromium is required for the packaged browser init smoke test');
}

async function serve(directory) {
  const server = createServer(async (request, response) => {
    try {
      const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
      const relative = pathname === '/' ? 'browser-smoke.html' : pathname.slice(1);
      const file = resolve(directory, relative);
      if (file !== directory && !file.startsWith(`${directory}${sep}`)) {
        response.writeHead(403).end('forbidden');
        return;
      }

      const body = await readFile(file);
      const mime = {
        '.html': 'text/html; charset=utf-8',
        '.js': 'text/javascript; charset=utf-8',
        '.json': 'application/json; charset=utf-8',
        '.wasm': 'application/wasm',
      }[extname(file)] ?? 'application/octet-stream';
      response.writeHead(200, { 'Content-Type': mime });
      response.end(body);
    } catch {
      response.writeHead(404).end('not found');
    }
  });

  await new Promise((resolveListen, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolveListen);
  });
  return server;
}

async function runChrome(chrome, url, profile) {
  const child = spawn(
    chrome,
    [
      '--headless',
      '--disable-gpu',
      '--no-sandbox',
      '--disable-background-networking',
      '--disable-component-update',
      '--disable-default-apps',
      '--disable-extensions',
      '--disable-sync',
      '--no-first-run',
      '--no-default-browser-check',
      '--remote-debugging-port=0',
      `--user-data-dir=${profile}`,
      url,
    ],
    { stdio: ['ignore', 'ignore', 'pipe'] },
  );

  let stderr = '';
  child.stderr.on('data', (chunk) => {
    stderr += chunk;
  });

  const sleep = (milliseconds) =>
    new Promise((resolveSleep) => setTimeout(resolveSleep, milliseconds));
  let socket;
  try {
    const portFile = join(profile, 'DevToolsActivePort');
    let port;
    for (let attempt = 0; attempt < 300; attempt += 1) {
      if (child.exitCode !== null) {
        throw new Error(`Chrome exited with ${child.exitCode}\n${stderr}`);
      }
      try {
        [port] = (await readFile(portFile, 'utf8')).trim().split('\n');
        if (port) break;
      } catch {
        // Chrome has not opened its DevTools endpoint yet.
      }
      await sleep(100);
    }
    if (!port) throw new Error(`Chrome DevTools endpoint timed out\n${stderr}`);

    let target;
    for (let attempt = 0; attempt < 100; attempt += 1) {
      const targets = await fetch(`http://127.0.0.1:${port}/json/list`).then((response) =>
        response.json(),
      );
      target = targets.find((candidate) => candidate.type === 'page' && candidate.url === url);
      if (target) break;
      await sleep(100);
    }
    if (!target) throw new Error(`browser smoke page did not open\n${stderr}`);

    socket = new WebSocket(target.webSocketDebuggerUrl);
    await new Promise((resolveOpen, rejectOpen) => {
      socket.once('open', resolveOpen);
      socket.once('error', rejectOpen);
    });

    let nextId = 1;
    const pending = new Map();
    socket.on('message', (message) => {
      const response = JSON.parse(message.toString());
      if (!response.id) return;
      const callbacks = pending.get(response.id);
      if (!callbacks) return;
      pending.delete(response.id);
      if (response.error) callbacks.reject(new Error(response.error.message));
      else callbacks.resolve(response.result);
    });
    const command = (method, params = {}) =>
      new Promise((resolveCommand, rejectCommand) => {
        const id = nextId;
        nextId += 1;
        pending.set(id, { resolve: resolveCommand, reject: rejectCommand });
        socket.send(JSON.stringify({ id, method, params }));
      });

    await command('Runtime.enable');
    for (let attempt = 0; attempt < 300; attempt += 1) {
      const evaluation = await command('Runtime.evaluate', {
        expression:
          "({status: document.documentElement.dataset.ojsSmoke, body: document.body.textContent})",
        returnByValue: true,
      });
      const value = evaluation.result.value;
      if (value?.status === 'ok') return;
      if (value?.status === 'failed') {
        throw new Error(`packaged browser init failed: ${value.body}`);
      }
      await sleep(100);
    }
    throw new Error(`packaged browser init smoke test timed out\n${stderr}`);
  } finally {
    if (socket?.readyState === WebSocket.OPEN) socket.close();
    if (child.exitCode === null) {
      child.kill('SIGTERM');
      await Promise.race([
        new Promise((resolveClose) => child.once('close', resolveClose)),
        sleep(2_000),
      ]);
      if (child.exitCode === null) child.kill('SIGKILL');
    }
  }
}

await mkdir(consumer, { recursive: true });

try {
  let tarball;
  if (process.argv[2]) {
    tarball = resolve(root, process.argv[2]);
    await access(tarball);
  } else {
    const packed = JSON.parse(
      run(
        'npm',
        ['pack', '--ignore-scripts', '--json', '--pack-destination', scratch],
        root,
        { capture: true },
      ),
    );
    tarball = join(scratch, packed[0].filename);
  }

  await writeFile(
    join(consumer, 'package.json'),
    JSON.stringify({ private: true, type: 'module' }, null, 2),
  );
  run(
    'npm',
    ['install', '--ignore-scripts', '--no-audit', '--no-fund', tarball],
    consumer,
  );

  await writeFile(
    join(consumer, 'import-smoke.mjs'),
    `import init, * as sdk from '@openjobspec/wasm';
const required = [
  'OJSClient',
  'EdgeClient',
  'CloudflareClient',
  'DenoClient',
  'VercelEdgeClient',
  'ServiceWorkerClient',
  'EncryptionCodec',
  'SchemaValidator',
  'background_sync_tag_prefix',
];
if (typeof init !== 'function') throw new Error('default init export is missing');
for (const name of required) {
  if (!(name in sdk)) throw new Error(\`missing export: \${name}\`);
}
`,
  );
  run(process.execPath, ['import-smoke.mjs'], consumer);

  await writeFile(
    join(consumer, 'types-smoke.ts'),
    `import init, {
  OJSClient,
  EdgeClient,
  CloudflareClient,
  DenoClient,
  VercelEdgeClient,
  ServiceWorkerClient,
  EncryptionCodec,
  SchemaValidator,
  background_sync_tag_prefix,
} from '@openjobspec/wasm';

void init();
new OJSClient('https://example.com');
new EdgeClient('https://example.com');
new CloudflareClient('https://example.com');
new DenoClient('https://example.com');
new VercelEdgeClient('https://example.com');
new ServiceWorkerClient('https://example.com');
new EncryptionCodec();
new SchemaValidator();
const prefix: string = background_sync_tag_prefix();
void prefix;
`,
  );
  await writeFile(
    join(consumer, 'tsconfig.json'),
    JSON.stringify(
      {
        compilerOptions: {
          lib: ['DOM', 'ES2022', 'ESNext.Disposable'],
          module: 'ESNext',
          moduleResolution: 'Bundler',
          noEmit: true,
          strict: true,
          target: 'ES2022',
        },
        files: ['types-smoke.ts'],
      },
      null,
      2,
    ),
  );
  const tsc = join(root, 'node_modules', 'typescript', 'bin', 'tsc');
  await access(tsc);
  run(process.execPath, [tsc, '--project', join(consumer, 'tsconfig.json')], root);

  await writeFile(
    join(consumer, 'browser-smoke.html'),
    `<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <script type="importmap">
      {"imports":{"@openjobspec/wasm":"/node_modules/@openjobspec/wasm/pkg/ojs_wasm_sdk.js"}}
    </script>
  </head>
  <body>
    <script type="module">
      try {
        const sdk = await import('@openjobspec/wasm');
        await sdk.default();
        new sdk.OJSClient('https://example.com');
        document.documentElement.dataset.ojsSmoke = 'ok';
        document.title = 'OJS_WASM_SMOKE_OK';
      } catch (error) {
        document.documentElement.dataset.ojsSmoke = 'failed';
        document.body.textContent = String(error?.stack || error);
        document.title = 'OJS_WASM_SMOKE_FAILED';
      }
    </script>
  </body>
</html>
`,
  );

  const chrome = await findChrome();
  const server = await serve(consumer);
  try {
    const address = server.address();
    await runChrome(
      chrome,
      `http://127.0.0.1:${address.port}/browser-smoke.html`,
      join(scratch, 'chrome-profile'),
    );
  } finally {
    await new Promise((resolveClose, reject) => {
      server.close((error) => (error ? reject(error) : resolveClose()));
    });
  }

  console.log(`Package smoke passed: ${tarball}`);
} finally {
  await rm(scratch, { recursive: true, force: true });
}
