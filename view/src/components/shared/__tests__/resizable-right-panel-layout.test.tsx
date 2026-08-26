import { ResizableRightPanelLayout } from '@/components/shared/resizable-right-panel-layout';
import { type ResizableRightPanelType } from '@/lib/right-panel.utils';
import { render, screen } from '@testing-library/react';
import { type ComponentProps, type ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('@/components/ui/resizable', () => ({
  ResizablePanelGroup: ({ children }: { children: ReactNode }) => (
    <div data-testid="panel-group">{children}</div>
  ),
  ResizableHandle: ({
    'aria-label': ariaLabel,
    className,
  }: {
    'aria-label': string;
    className?: string;
  }) => <div role="separator" aria-label={ariaLabel} className={className} />,
  ResizablePanel: ({
    children,
    defaultSize,
    ...props
  }: ComponentProps<'div'> & { defaultSize?: number | string }) => (
    <div data-default-size={defaultSize} {...props}>
      {children}
    </div>
  ),
}));

describe('ResizableRightPanelLayout', () => {
  it.each<{
    name: string;
    panelType: ResizableRightPanelType;
    defaultSize: number | string;
  }>([
    {
      name: 'active decisions',
      panelType: 'activeDecisions',
      defaultSize: 320,
    },
    { name: 'in-call chat', panelType: 'callChat', defaultSize: 380 },
    { name: 'in-call decisions', panelType: 'callDecisions', defaultSize: 380 },
    { name: 'reply thread', panelType: 'thread', defaultSize: 480 },
    { name: 'selected forum post', panelType: 'forumPost', defaultSize: 720 },
  ])(
    'gives the $name panel the shared resize control and its default width',
    ({ panelType, defaultSize }) => {
      render(
        <ResizableRightPanelLayout
          panelType={panelType}
          panel={<aside>Panel content</aside>}
          resizeHandleLabel="Resize right panel"
        >
          <main>Main content</main>
        </ResizableRightPanelLayout>,
      );

      const resizeHandle = screen.getByRole('separator', {
        name: 'Resize right panel',
      });
      expect(resizeHandle).toHaveClass(
        'cursor-col-resize',
        'before:transition-[width]',
        'hover:before:w-0.75',
        'data-[separator=active]:before:w-0.75',
      );
      expect(document.querySelector('[data-default-size]')).toHaveAttribute(
        'data-default-size',
        String(defaultSize),
      );
      expect(screen.getByText('Panel content')).toBeInTheDocument();
    },
  );

  it('does not add a resize control when the right panel is closed', () => {
    render(
      <ResizableRightPanelLayout
        panelType="activeDecisions"
        resizeHandleLabel="Resize right panel"
      >
        <main>Main content</main>
      </ResizableRightPanelLayout>,
    );

    expect(screen.queryByRole('separator')).not.toBeInTheDocument();
    expect(screen.getByText('Main content')).toBeInTheDocument();
  });

  it('restores a saved width for the matching panel type', () => {
    vi.mocked(localStorage.getItem).mockReturnValue(
      JSON.stringify({ thread: 612, forumPost: 840 }),
    );

    render(
      <ResizableRightPanelLayout
        panelType="thread"
        panel={<aside>Thread</aside>}
        resizeHandleLabel="Resize right panel"
      >
        <main>Main content</main>
      </ResizableRightPanelLayout>,
    );

    expect(document.querySelector('[data-default-size]')).toHaveAttribute(
      'data-default-size',
      '612',
    );
  });
});
