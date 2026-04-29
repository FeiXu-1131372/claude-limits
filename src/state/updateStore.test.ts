import { describe, it, expect, beforeEach } from 'vitest';
import { useUpdateStore } from './updateStore';

describe('updateStore', () => {
  beforeEach(() => {
    useUpdateStore.setState({
      status: 'idle',
      version: null,
      progress: 0,
      error: null,
      lastCheckedAt: null,
    });
  });

  it('starts idle', () => {
    expect(useUpdateStore.getState().status).toBe('idle');
  });

  it('transitions to checking', () => {
    useUpdateStore.getState().setStatus('checking');
    expect(useUpdateStore.getState().status).toBe('checking');
  });

  it('records up-to-date with timestamp', () => {
    useUpdateStore.getState().setUpToDate('2026-04-29T12:00:00Z');
    const s = useUpdateStore.getState();
    expect(s.status).toBe('up-to-date');
    expect(s.lastCheckedAt).toBe('2026-04-29T12:00:00Z');
  });

  it('records available with version', () => {
    useUpdateStore.getState().setAvailable('0.2.0');
    const s = useUpdateStore.getState();
    expect(s.status).toBe('available');
    expect(s.version).toBe('0.2.0');
  });

  it('updates progress while downloading', () => {
    useUpdateStore.getState().setProgress(0.42);
    const s = useUpdateStore.getState();
    expect(s.status).toBe('downloading');
    expect(s.progress).toBeCloseTo(0.42);
  });

  it('records ready with version and clears progress', () => {
    useUpdateStore.getState().setProgress(0.99);
    useUpdateStore.getState().setReady('0.2.0');
    const s = useUpdateStore.getState();
    expect(s.status).toBe('ready');
    expect(s.version).toBe('0.2.0');
    expect(s.progress).toBe(1);
  });

  it('records failed with phase + message and preserves version', () => {
    useUpdateStore.getState().setReady('0.2.0');
    useUpdateStore.getState().setFailed('install', 'file in use');
    const s = useUpdateStore.getState();
    expect(s.status).toBe('failed');
    expect(s.error).toEqual({ phase: 'install', message: 'file in use' });
    expect(s.version).toBe('0.2.0'); // retained so retry banner can show it
  });

  it('reset returns to idle and clears error', () => {
    useUpdateStore.getState().setFailed('check', 'no network');
    useUpdateStore.getState().reset();
    const s = useUpdateStore.getState();
    expect(s.status).toBe('idle');
    expect(s.error).toBeNull();
  });
});
