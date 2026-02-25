/**
 * React hook for the OJS WASM SDK.
 *
 * Initializes the WASM module once and provides enqueue/getJob/cancelJob methods.
 *
 * Usage:
 *   const { ready, enqueue, getJob } = useOJS('http://localhost:8080');
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import init, { OJSClient } from '@openjobspec/wasm';
import type { Job, EnqueueOptions } from '@openjobspec/wasm';

export type { Job, EnqueueOptions };

export function useOJS(serverUrl: string) {
  const clientRef = useRef<InstanceType<typeof OJSClient> | null>(null);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let cancelled = false;
    init()
      .then(() => {
        if (!cancelled) {
          clientRef.current = new OJSClient(serverUrl);
          setReady(true);
        }
      })
      .catch((err) => {
        if (!cancelled) setError(err);
      });
    return () => {
      cancelled = true;
    };
  }, [serverUrl]);

  const enqueue = useCallback(
    async (type: string, args: unknown[], options?: EnqueueOptions): Promise<Job> => {
      if (!clientRef.current) throw new Error('OJS not initialized');
      return options
        ? clientRef.current.enqueue_with_options(type, args, options)
        : clientRef.current.enqueue(type, args);
    },
    [],
  );

  const getJob = useCallback(async (id: string): Promise<Job> => {
    if (!clientRef.current) throw new Error('OJS not initialized');
    return clientRef.current.get_job(id);
  }, []);

  const cancelJob = useCallback(async (id: string): Promise<Job> => {
    if (!clientRef.current) throw new Error('OJS not initialized');
    return clientRef.current.cancel_job(id);
  }, []);

  const health = useCallback(async () => {
    if (!clientRef.current) throw new Error('OJS not initialized');
    return clientRef.current.health();
  }, []);

  return { ready, error, enqueue, getJob, cancelJob, health };
}

