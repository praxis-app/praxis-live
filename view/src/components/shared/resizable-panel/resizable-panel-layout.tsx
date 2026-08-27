import { ResizablePanelHandle } from '@/components/shared/resizable-panel/resizable-panel-handle';
import { ResizablePanel, ResizablePanelGroup } from '@/components/ui/resizable';
import {
  getStoredPanelWidth,
  type ResizablePanelType,
  savePanelWidth,
} from '@/lib/panel-width.utils';
import { type ReactNode, useRef } from 'react';

interface PanelConfig {
  defaultSize: number | string;
  minSize: number | string;
  maxSize: number | string;
  position: 'left' | 'right';
  groupResizeBehavior?: 'preserve-pixel-size';
}

const PANEL_CONFIG: Record<ResizablePanelType, PanelConfig> = {
  activeDecisions: {
    defaultSize: 320,
    minSize: '18rem',
    maxSize: '70%',
    position: 'right',
  },
  callChat: {
    defaultSize: 380,
    minSize: '18rem',
    maxSize: '70%',
    position: 'right',
  },
  callDecisions: {
    defaultSize: 380,
    minSize: '18rem',
    maxSize: '70%',
    position: 'right',
  },
  channelsList: {
    defaultSize: 240,
    minSize: '12rem',
    maxSize: 400,
    position: 'left',
    groupResizeBehavior: 'preserve-pixel-size',
  },
  thread: {
    defaultSize: 480,
    minSize: '18rem',
    maxSize: '70%',
    position: 'right',
  },
  forumPost: {
    defaultSize: 720,
    minSize: '18rem',
    maxSize: '70%',
    position: 'right',
  },
};

interface Props {
  children: ReactNode;
  panel?: ReactNode;
  panelType: ResizablePanelType;
  resizeHandleLabel: string;
}

export const ResizablePanelLayout = ({
  children,
  panel,
  panelType,
  resizeHandleLabel,
}: Props) => {
  const panelRef = useRef<HTMLDivElement>(null);

  if (!panel) {
    return children;
  }

  const config = PANEL_CONFIG[panelType];
  const contentPanel = (
    <ResizablePanel className="h-full min-h-0 min-w-0" minSize="20rem">
      {children}
    </ResizablePanel>
  );
  const resizablePanel = (
    <ResizablePanel
      className="h-full min-h-0 min-w-0"
      elementRef={panelRef}
      defaultSize={getStoredPanelWidth(panelType) ?? config.defaultSize}
      minSize={config.minSize}
      maxSize={config.maxSize}
      groupResizeBehavior={config.groupResizeBehavior}
    >
      {panel}
    </ResizablePanel>
  );
  const resizeHandle = <ResizablePanelHandle label={resizeHandleLabel} />;

  return (
    <ResizablePanelGroup
      key={panelType}
      orientation="horizontal"
      onLayoutChanged={(_, { isUserInteraction }) => {
        if (isUserInteraction && panelRef.current) {
          savePanelWidth(
            panelType,
            panelRef.current.getBoundingClientRect().width,
          );
        }
      }}
    >
      {config.position === 'left' ? resizablePanel : contentPanel}
      {resizeHandle}
      {config.position === 'left' ? contentPanel : resizablePanel}
    </ResizablePanelGroup>
  );
};
