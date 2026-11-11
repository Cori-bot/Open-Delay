import { useCallback, useEffect, useRef, useState } from 'react';
import { api, isDesktop, statusEvents } from './api';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { parseCommandResult } from './validation';
import type { Command, Preferences, RouteConfig, Status } from './types';
export function useEngine() {
  const [status, setStatus] = useState<Status | null>(null);
  const [preferences, setPreferences] = useState<Preferences | null>(null);
  const [routes, setRoutes] = useState<RouteConfig[]>([]);
  const [error, setError] = useState('');
  const [transportError, setTransportError] = useState('');
  const [nativeError, setNativeError] = useState('');
  const [busy, setBusy] = useState(false);
  const pendingCommands = useRef(0);
  const commandQueue = useRef<Promise<void>>(Promise.resolve());
  const latestCommand = useRef(0);
  const mounted = useRef(false);
  const lifecycle = useRef(0);
  const refreshId = useRef(0);
  const refreshController = useRef<AbortController | null>(null);
  const refresh = useCallback(async () => {
    const epoch = lifecycle.current;
    const requestId = ++refreshId.current;
    refreshController.current?.abort();
    const controller = new AbortController(); refreshController.current = controller;
    const [nextStatus, nextRoutes, nextPreferences] = await Promise.all([api.status(controller.signal), api.routes(controller.signal), api.preferences(controller.signal)]);
    if (!mounted.current || controller.signal.aborted || lifecycle.current !== epoch || requestId !== refreshId.current) return;
    setStatus(old => !old || nextStatus.sequence >= old.sequence || nextStatus.endpoints?.api_base !== old.endpoints?.api_base ? nextStatus : old);
    setRoutes(nextRoutes); setPreferences(nextPreferences); setTransportError('');
  }, []);
  const reconnect=useCallback(async()=>{setStatus(null);await refresh();},[refresh]);
  useEffect(() => {
    mounted.current = true;
    lifecycle.current += 1;
    const abort = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    let stopped = false;
    async function poll() {
      try {
        const next = await api.status(abort.signal);
        if (!stopped) { setStatus(old => !old || next.sequence >= old.sequence ? next : old); setPreferences(old => JSON.stringify(old) === JSON.stringify(next.preferences) ? old : next.preferences); setTransportError(''); }
      } catch (cause) { if (!stopped) { setStatus(null); setTransportError(cause instanceof Error ? cause.message : 'Connection unavailable'); } }
      if (!stopped) timer = setTimeout(poll, document.hidden ? 5000 : 1000);
    }
    void refresh().catch(cause => { if (!stopped) setTransportError(cause instanceof Error ? cause.message : 'Connection unavailable'); });
    void statusEvents(abort.signal, next => { if (!stopped) { setStatus(old => !old || next.sequence >= old.sequence ? next : old); setPreferences(old => JSON.stringify(old) === JSON.stringify(next.preferences) ? old : next.preferences); setTransportError(''); } }).catch(() => { if (!stopped) { setStatus(null); void poll(); } });
    return () => { stopped = true; mounted.current = false; lifecycle.current += 1; refreshController.current?.abort(); abort.abort(); clearTimeout(timer); };
  }, [refresh]);
  useEffect(() => {
    if (!isDesktop()) return;
    let stopped = false;
    const remove: (() => void)[] = [];
    let nativeRevision = 0;
    const desktopState = (value: unknown) => {
      if (!value || typeof value !== 'object' || !('errors' in value) || !Array.isArray(value.errors) || !value.errors.every((entry: unknown) => typeof entry === 'string')) throw new Error('Invalid desktop status');
      if (!stopped) setNativeError(value.errors.join(' · '));
    };
    async function attach() {
      const statusRemove = await listen<unknown>('desktop-status', event => { nativeRevision++; try { desktopState(event.payload); } catch (cause) { if (!stopped) setNativeError(String(cause)); } });
      if (stopped) { statusRemove(); return; } remove.push(statusRemove);
      const commandRemove = await listen<unknown>('command-result', event => {
        if (stopped) return;
        try {
          const result = parseCommandResult(event.payload);
          setStatus(old => !old || result.status.sequence >= old.sequence ? { ...result.status, endpoints: result.status.endpoints ?? old?.endpoints } : old);
          if (!result.success) setError([result.message ?? result.reason, ...(result.results ?? []).filter(route => !route.success).map(route => `${route.route_id}: ${route.reason ?? 'refused'}`)].filter(Boolean).join(' · ') || 'Native command refused');
        } catch (cause) { setError(String(cause)); }
      });
      if (stopped) { commandRemove(); return; } remove.push(commandRemove);
      const revision = nativeRevision;
      const initial = await invoke('desktop_status');
      if (!stopped && revision === nativeRevision) desktopState(initial);
    }
    void attach().catch(cause => { if (!stopped) setNativeError(String(cause)); });
    return () => { stopped = true; remove.forEach(unlisten => unlisten()); };
  }, []);
  const run = useCallback(async (command: Omit<Command, 'id'>) => {
    const commandNumber = ++latestCommand.current;
    const preceding = commandQueue.current;
    let release!: () => void;
    commandQueue.current = new Promise<void>(resolve => { release = resolve; });
    pendingCommands.current += 1; setBusy(true); setError('');
    try {
      // Dispatch in click order: a pending delay request must not arrive after Go Live.
      await preceding;
      const result = await api.command(command);
      if (mounted.current) {
        setStatus(old => !old || result.status.sequence >= old.sequence ? { ...result.status, endpoints: result.status.endpoints ?? old?.endpoints } : old);
        if (!result.success && commandNumber === latestCommand.current) setError([result.message ?? result.reason, ...(result.results ?? []).filter(route => !route.success).map(route => `${route.route_id}: ${route.reason ?? 'refused'}`)].join(' · '));
      }
    } catch (cause) {
      if (mounted.current && commandNumber === latestCommand.current) setError(cause instanceof Error ? cause.message : 'Command failed');
    } finally {
      release();
      pendingCommands.current = Math.max(0, pendingCommands.current - 1);
      if (mounted.current) setBusy(pendingCommands.current > 0);
    }
  }, []);
  useEffect(() => {
    if (isDesktop() || !preferences) return;
    const keydown = (event: KeyboardEvent) => {
      const target = event.target;
      if (event.repeat || target instanceof HTMLElement && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT', 'BUTTON'].includes(target.tagName))) return;
      const binding = [event.ctrlKey ? 'Ctrl' : '', event.altKey ? 'Alt' : '', event.shiftKey ? 'Shift' : '', event.metaKey ? 'Meta' : '', event.key.length === 1 ? event.key.toUpperCase() : event.key].filter(Boolean).join('+');
      const keys = preferences.hotkeys;
      if (binding === keys.go_live) { event.preventDefault(); void run({ action: 'go_live' }); }
      else if (binding === keys.toggle_overlay) { event.preventDefault(); void run({ action: 'toggle_overlay' }); }
      else if (binding === keys.custom_delay) { event.preventDefault(); void run({ action: 'prepare_delay', delay_ms: keys.custom_delay_ms }); }
      else if (binding === keys.start_delay) { event.preventDefault(); void run({ action: 'prepare_delay', delay_ms: preferences.buffer_ms }); }
    };
    document.addEventListener('keydown', keydown);
    return () => document.removeEventListener('keydown', keydown);
  }, [preferences, run]);
  return { status, preferences, routes, error: error || transportError || nativeError, transportError, busy, run, refresh, reconnect, setError };
}
