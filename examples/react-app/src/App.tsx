/**
 * OJS WASM SDK — React Example App
 *
 * Demonstrates:
 * - Initializing the WASM SDK with a custom React hook
 * - Enqueuing jobs from a form
 * - Polling job status
 * - Displaying job results
 */

import { useState } from 'react';
import { useOJS } from './useOJS';
import type { Job } from './useOJS';

const OJS_URL = import.meta.env.VITE_OJS_URL || 'http://localhost:8080';

function App() {
  const { ready, error, enqueue, getJob, health } = useOJS(OJS_URL);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [status, setStatus] = useState<string>('');

  async function handleEnqueue(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const form = new FormData(e.currentTarget);
    const type = form.get('type') as string;
    const argsRaw = form.get('args') as string;

    try {
      const args = JSON.parse(argsRaw || '[]');
      const job = await enqueue(type, args);
      setJobs((prev) => [job, ...prev]);
      setStatus(`Enqueued job ${job.id}`);
    } catch (err) {
      setStatus(`Error: ${err}`);
    }
  }

  async function handleRefresh(jobId: string) {
    try {
      const job = await getJob(jobId);
      setJobs((prev) => prev.map((j) => (j.id === jobId ? job : j)));
    } catch (err) {
      setStatus(`Error refreshing: ${err}`);
    }
  }

  async function handleHealthCheck() {
    try {
      const h = await health();
      setStatus(`Server: ${h.status} (v${h.version || 'unknown'})`);
    } catch (err) {
      setStatus(`Health check failed: ${err}`);
    }
  }

  if (error) return <div style={{ color: 'red' }}>Failed to load WASM: {error.message}</div>;
  if (!ready) return <div>Loading OJS WASM SDK...</div>;

  return (
    <div style={{ maxWidth: 640, margin: '2rem auto', fontFamily: 'system-ui' }}>
      <h1>OJS WASM + React</h1>
      <p>Connected to <code>{OJS_URL}</code></p>

      <button onClick={handleHealthCheck} style={{ marginBottom: '1rem' }}>
        Health Check
      </button>
      {status && <p><em>{status}</em></p>}

      <h2>Enqueue Job</h2>
      <form onSubmit={handleEnqueue}>
        <div style={{ marginBottom: '0.5rem' }}>
          <label>Type: </label>
          <input name="type" defaultValue="email.send" required />
        </div>
        <div style={{ marginBottom: '0.5rem' }}>
          <label>Args (JSON): </label>
          <input name="args" defaultValue='["user@example.com", "Hello!"]' style={{ width: 300 }} />
        </div>
        <button type="submit">Enqueue</button>
      </form>

      <h2>Jobs ({jobs.length})</h2>
      <table style={{ width: '100%', borderCollapse: 'collapse' }}>
        <thead>
          <tr>
            <th style={{ textAlign: 'left', borderBottom: '1px solid #ccc' }}>ID</th>
            <th style={{ textAlign: 'left', borderBottom: '1px solid #ccc' }}>Type</th>
            <th style={{ textAlign: 'left', borderBottom: '1px solid #ccc' }}>State</th>
            <th style={{ borderBottom: '1px solid #ccc' }}></th>
          </tr>
        </thead>
        <tbody>
          {jobs.map((job) => (
            <tr key={job.id}>
              <td style={{ fontFamily: 'monospace', fontSize: '0.85rem' }}>
                {job.id.substring(0, 8)}…
              </td>
              <td>{job.type}</td>
              <td>
                <strong>{job.state || 'enqueued'}</strong>
              </td>
              <td>
                <button onClick={() => handleRefresh(job.id)}>Refresh</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export default App;
