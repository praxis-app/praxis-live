import { LocalStorageKeys } from '@/constants/shared.constants';
import {
  getStoredRightPanelWidth,
  saveRightPanelWidth,
} from '@/lib/right-panel.utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';

describe('right panel width storage', () => {
  beforeEach(() => {
    vi.mocked(localStorage.getItem).mockReturnValue(null);
  });

  it('stores every panel width in one local storage object', () => {
    vi.mocked(localStorage.getItem).mockReturnValue(
      JSON.stringify({ activeDecisions: 360 }),
    );

    saveRightPanelWidth('thread', 511.6);

    expect(localStorage.setItem).toHaveBeenCalledWith(
      LocalStorageKeys.RightPanelWidths,
      JSON.stringify({ activeDecisions: 360, thread: 512 }),
    );
  });

  it('returns only a finite saved width for the requested panel', () => {
    vi.mocked(localStorage.getItem).mockReturnValue(
      JSON.stringify({ thread: 512, forumPost: 'wide' }),
    );

    expect(getStoredRightPanelWidth('thread')).toBe(512);
    expect(getStoredRightPanelWidth('forumPost')).toBeNull();
    expect(getStoredRightPanelWidth('callChat')).toBeNull();
  });
});
