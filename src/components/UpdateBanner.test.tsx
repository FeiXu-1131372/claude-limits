import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { UpdateBanner } from './UpdateBanner';
import { useUpdateStore } from '../state/updateStore';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

describe('UpdateBanner', () => {
  beforeEach(() => {
    useUpdateStore.setState({
      status: 'idle',
      version: null,
      progress: 0,
      error: null,
      lastCheckedAt: null,
    });
    vi.clearAllMocks();
  });

  it('renders nothing when idle', () => {
    const { container } = render(<UpdateBanner />);
    expect(container.firstChild).toBeNull();
  });

  it('renders nothing when checking / available / downloading', () => {
    for (const status of ['checking', 'available', 'downloading'] as const) {
      useUpdateStore.setState({ status, version: '0.2.0' });
      const { container, unmount } = render(<UpdateBanner />);
      expect(container.firstChild).toBeNull();
      unmount();
    }
  });

  it('renders the install banner when ready', () => {
    useUpdateStore.setState({ status: 'ready', version: '0.2.0' });
    render(<UpdateBanner />);
    expect(screen.getByText(/Update ready · v0\.2\.0/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Install & restart/ })).toBeInTheDocument();
  });

  it('renders the retry banner when install failed', () => {
    useUpdateStore.setState({
      status: 'failed',
      version: '0.2.0',
      error: { phase: 'install', message: 'file in use' },
    });
    render(<UpdateBanner />);
    expect(screen.getByText(/Install failed/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Retry/ })).toBeInTheDocument();
  });

  it('renders nothing for failed checks (only install failures surface)', () => {
    useUpdateStore.setState({
      status: 'failed',
      error: { phase: 'check', message: 'no network' },
    });
    const { container } = render(<UpdateBanner />);
    expect(container.firstChild).toBeNull();
  });

  it('invokes install_update when user clicks Install', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    useUpdateStore.setState({ status: 'ready', version: '0.2.0' });
    render(<UpdateBanner />);
    fireEvent.click(screen.getByRole('button', { name: /Install & restart/ }));
    expect(invoke).toHaveBeenCalledWith('install_update');
  });
});
