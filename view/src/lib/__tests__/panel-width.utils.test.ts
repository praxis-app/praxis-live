import { LocalStorageKeys } from '@/constants/shared.constants';
import { getStoredPanelWidth, savePanelWidth } from '@/lib/panel-width.utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';

describe('panel width storage', () => {
  beforeEach(() => {
    vi.mocked(localStorage.getItem).mockReturnValue(null);
  });

  it('uses a key that represents panels on either side', () => {
    expect(LocalStorageKeys.PanelWidths).toBe('panel-widths');
  });

  it('stores left and right panel widths in one local storage object', () => {
    vi.mocked(localStorage.getItem).mockReturnValue(
      JSON.stringify({ activeDecisions: 360 }),
    );

    savePanelWidth('channelsList', 255.6);

    expect(localStorage.setItem).toHaveBeenCalledWith(
      LocalStorageKeys.PanelWidths,
      JSON.stringify({ activeDecisions: 360, channelsList: 256 }),
    );
  });

  it('returns only a finite saved width for the requested panel', () => {
    vi.mocked(localStorage.getItem).mockReturnValue(
      JSON.stringify({ thread: 512, forumPost: 'wide' }),
    );

    expect(getStoredPanelWidth('thread')).toBe(512);
    expect(getStoredPanelWidth('forumPost')).toBeNull();
    expect(getStoredPanelWidth('channelsList')).toBeNull();
  });
});
