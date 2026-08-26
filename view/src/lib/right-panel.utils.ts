import { LocalStorageKeys } from '@/constants/shared.constants';

export type ResizableRightPanelType =
  | 'activeDecisions'
  | 'callChat'
  | 'callDecisions'
  | 'thread'
  | 'forumPost';

type RightPanelWidths = Partial<Record<ResizableRightPanelType, number>>;

const getStoredRightPanelWidths = (): RightPanelWidths => {
  try {
    const stored = JSON.parse(
      localStorage.getItem(LocalStorageKeys.RightPanelWidths) || '{}',
    ) as unknown;
    return stored && typeof stored === 'object'
      ? (stored as RightPanelWidths)
      : {};
  } catch {
    return {};
  }
};

export const getStoredRightPanelWidth = (
  panelType: ResizableRightPanelType,
) => {
  const width = getStoredRightPanelWidths()[panelType];
  return typeof width === 'number' && Number.isFinite(width) ? width : null;
};

export const saveRightPanelWidth = (
  panelType: ResizableRightPanelType,
  width: number,
) => {
  localStorage.setItem(
    LocalStorageKeys.RightPanelWidths,
    JSON.stringify({
      ...getStoredRightPanelWidths(),
      [panelType]: Math.round(width),
    }),
  );
};
