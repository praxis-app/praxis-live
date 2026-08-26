import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import {
  getStoredRightPanelWidth,
  type ResizableRightPanelType,
  saveRightPanelWidth,
} from '@/lib/right-panel.utils';
import { type ReactNode, useRef } from 'react';

const RIGHT_PANEL_DEFAULT_SIZES = {
  activeDecisions: 320,
  callChat: 380,
  callDecisions: 380,
  thread: 480,
  forumPost: 720,
} as const;

interface Props {
  children: ReactNode;
  panel?: ReactNode;
  panelType: ResizableRightPanelType;
  resizeHandleLabel: string;
}

export const ResizableRightPanelLayout = ({
  children,
  panel,
  panelType,
  resizeHandleLabel,
}: Props) => {
  const rightPanelRef = useRef<HTMLDivElement>(null);

  if (!panel) {
    return children;
  }

  return (
    <ResizablePanelGroup
      key={panelType}
      orientation="horizontal"
      onLayoutChanged={(_, { isUserInteraction }) => {
        if (isUserInteraction && rightPanelRef.current) {
          saveRightPanelWidth(
            panelType,
            rightPanelRef.current.getBoundingClientRect().width,
          );
        }
      }}
    >
      <ResizablePanel className="h-full min-h-0 min-w-0" minSize="20rem">
        {children}
      </ResizablePanel>
      <ResizableHandle
        aria-label={resizeHandleLabel}
        className="before:bg-border cursor-col-resize bg-transparent before:absolute before:inset-y-0 before:left-1/2 before:w-px before:-translate-x-1/2 before:transition-[width] before:duration-150 before:ease-out hover:before:w-0.75 focus-visible:before:w-0.75 data-[separator=active]:before:w-0.75 motion-reduce:before:transition-none"
        data-testid="right-panel-resize-handle"
      />
      <ResizablePanel
        className="h-full min-h-0 min-w-0"
        elementRef={rightPanelRef}
        defaultSize={
          getStoredRightPanelWidth(panelType) ??
          RIGHT_PANEL_DEFAULT_SIZES[panelType]
        }
        minSize="18rem"
        maxSize="70%"
      >
        {panel}
      </ResizablePanel>
    </ResizablePanelGroup>
  );
};
