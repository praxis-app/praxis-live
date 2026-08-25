import { useVotingDeadlineLabel } from '@/hooks/use-voting-deadline';
import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));
vi.mock('@/lib/time.utils', () => ({
  timeFromNow: vi.fn(() => 'Ends in 1 minute'),
  timeSinceEnded: vi.fn(() => 'Ended 1 minute ago'),
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

  it('reports how long ago the deadline passed', () => {
    const closingAt = new Date('2026-01-01T00:01:00Z').toISOString();
    const { result } = renderHook(() => useVotingDeadlineLabel(closingAt));

    act(() => {
      vi.advanceTimersByTime(61_000);
    });

    expect(result.current).toEqual({
      hasEnded: true,
      label: 'Ended 1 minute ago',
    });
  });

  it('reports a plain ended label when voting stopped before the deadline', () => {
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
