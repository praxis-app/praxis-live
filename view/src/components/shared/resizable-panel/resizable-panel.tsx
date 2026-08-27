import { ResizablePanelHandle } from '@/components/shared/resizable-panel/resizable-panel-handle';
import {
  ResizablePanel as ResizablePanelPrimitive,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import {
  getStoredPanelWidth,
  type ResizablePanelType,
  savePanelWidth,
} from '@/lib/panel-width.utils';
import { type ReactNode, useRef } from 'react';

interface Props {
  children: ReactNode;
  panel?: ReactNode;
  panelType: ResizablePanelType;
  resizeHandleLabel: string;
  defaultSize: number | string;
  minSize: number | string;
  maxSize: number | string;
  position: 'left' | 'right';
  groupResizeBehavior?: 'preserve-pixel-size';
}

export const ResizablePanel = ({
  children,
  panel,
  panelType,
  resizeHandleLabel,
  defaultSize,
  minSize,
  maxSize,
  position,
  groupResizeBehavior,
}: Props) => {
  const panelRef = useRef<HTMLDivElement>(null);

  if (!panel) {
    return children;
  }

  const contentPanel = (
    <ResizablePanelPrimitive
      className="h-full min-h-0 min-w-0"
      minSize="20rem"
    >
      {children}
    </ResizablePanelPrimitive>
  );
  const resizablePanel = (
    <ResizablePanelPrimitive
      className="h-full min-h-0 min-w-0"
      elementRef={panelRef}
      defaultSize={getStoredPanelWidth(panelType) ?? defaultSize}
      minSize={minSize}
      maxSize={maxSize}
      groupResizeBehavior={groupResizeBehavior}
    >
      {panel}
    </ResizablePanelPrimitive>
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
      {position === 'left' ? resizablePanel : contentPanel}
      {resizeHandle}
      {position === 'left' ? contentPanel : resizablePanel}
    </ResizablePanelGroup>
  );
};
