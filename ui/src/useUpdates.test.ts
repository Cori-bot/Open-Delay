import { describe, expect, it } from 'vitest';
import { parseUpdateStatus } from './useUpdates';

describe('native update status contract', () => {
  const status = { configured: true, phase: 'available', current_version: '0.2.0', version: '0.3.0', notes: 'Release notes', downloaded: 0, total: null, error: null };
  it('accepts available and indeterminate progress snapshots', () => {
    expect(parseUpdateStatus(status)).toEqual(status);
    expect(parseUpdateStatus({ ...status, phase: 'downloading', downloaded: 1024 })).toMatchObject({ downloaded: 1024, total: null });
  });
  it('rejects malformed phases, byte counts and release notes', () => {
    expect(() => parseUpdateStatus({ ...status, phase: 'execute' })).toThrow();
    expect(() => parseUpdateStatus({ ...status, downloaded: -1 })).toThrow();
    expect(() => parseUpdateStatus({ ...status, total: Infinity })).toThrow();
    expect(() => parseUpdateStatus({ ...status, notes: {} })).toThrow();
  });
});
