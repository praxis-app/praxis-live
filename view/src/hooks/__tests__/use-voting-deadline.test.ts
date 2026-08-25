import { useVotingDeadlineLabel } from '@/hooks/use-voting-deadline';
import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));
vi.mock('@/lib/time.utils', () => ({
  timeFromNow: vi.fn(() => 'Ends in 1 minute'),
}));

describe('useVotingDeadlineLabel', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-01-01T00:00:00Z'));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('counts down while the proposal is still open', () => {
    const closingAt = new Date('2026-01-01T00:05:00Z').toISOString();
    const { result } = renderHook(() => useVotingDeadlineLabel(closingAt));

    expect(result.current).toEqual({
      hasEnded: false,
      label: 'Ends in 1 minute',
    });
  });

  it('reports the deadline as ended once it passes', () => {
    const closingAt = new Date('2026-01-01T00:01:00Z').toISOString();
    const { result } = renderHook(() => useVotingDeadlineLabel(closingAt));

    act(() => {
      vi.advanceTimersByTime(61_000);
    });

    expect(result.current).toEqual({ hasEnded: true, label: 'time.ended' });
  });

  it('reports the deadline as ended when voting is no longer active', () => {
    const closingAt = new Date('2026-01-01T00:05:00Z').toISOString();
    const { result } = renderHook(() =>
      useVotingDeadlineLabel(closingAt, false),
    );

    expect(result.current).toEqual({ hasEnded: true, label: 'time.ended' });
  });

  it('has no label when there is no deadline', () => {
    const { result } = renderHook(() => useVotingDeadlineLabel(undefined));

    expect(result.current).toEqual({ hasEnded: false, label: null });
  });
});
