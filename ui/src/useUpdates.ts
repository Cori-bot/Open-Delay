import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { isDesktop } from './api';

const phases = ['idle', 'checking', 'available', 'up_to_date', 'downloading', 'installing', 'error'] as const;
export type UpdateStatus = { configured: boolean; phase: typeof phases[number]; current_version: string; version: string | null; notes: string | null; downloaded: number; total: number | null; error: string | null };
export function parseUpdateStatus(value: unknown): UpdateStatus {
  if (!value || typeof value !== 'object') throw new Error('Invalid update status');
  const v = value as Record<string, unknown>;
  if (typeof v.configured !== 'boolean' || !phases.includes(v.phase as UpdateStatus['phase']) || typeof v.current_version !== 'string' || ![v.version, v.notes, v.error].every(entry => entry === null || typeof entry === 'string') || typeof v.downloaded !== 'number' || !Number.isSafeInteger(v.downloaded) || v.downloaded < 0 || v.total !== null && (typeof v.total !== 'number' || !Number.isSafeInteger(v.total) || v.total < 0)) throw new Error('Invalid update status');
  return v as UpdateStatus;
}
export function useUpdates() {
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [error, setError] = useState('');
  const mounted = useRef(false);
  const generation = useRef(0);
  useEffect(() => {
    if (!isDesktop()) return;
    mounted.current = true;
    let stopped = false; let unlisten: (() => void) | undefined;
    async function start() {
      const remove = await listen<unknown>('update-status', event => {
        if (stopped) return;
        try { const next = parseUpdateStatus(event.payload); generation.current++; setStatus(next); }
        catch (cause) { setError(String(cause)); }
      });
      if (stopped) { remove(); return; }
      unlisten = remove;
      const epoch = generation.current;
      const initial = parseUpdateStatus(await invoke('update_status'));
      if (stopped) return;
      if (epoch === generation.current) setStatus(initial);
      if (epoch === generation.current && initial.configured && initial.phase === 'idle') {
        const checkEpoch = generation.current;
        const next = parseUpdateStatus(await invoke('check_update'));
        if (!stopped && checkEpoch === generation.current) setStatus(next);
      }
    }
    void start().catch(cause => { if (!stopped) setError(String(cause)); });
    return () => { stopped = true; mounted.current = false; unlisten?.(); };
  }, []);
  const run = async (command: 'check_update' | 'install_update') => {
    setError('');
    const epoch = generation.current;
    try { const next = parseUpdateStatus(await invoke(command)); if (mounted.current && epoch === generation.current) setStatus(next); }
    catch (cause) { if (mounted.current) setError(String(cause)); }
  };
  return { status, error, check: () => run('check_update'), install: () => run('install_update') };
}
