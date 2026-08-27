import { ResizablePanelLayout } from '@/components/shared/resizable-panel/resizable-panel-layout';
import { type ResizablePanelType } from '@/lib/panel-width.utils';
import { render, screen } from '@testing-library/react';
import { type ReactNode } from 'react';
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
  }: {
    children: ReactNode;
    defaultSize?: number | string;
  }) => (
    <div data-testid="panel" data-default-size={defaultSize}>
      {children}
    </div>
  ),
}));

describe('ResizablePanelLayout', () => {
  it.each<{
    name: string;
    panelType: ResizablePanelType;
    defaultSize: number;
    position: 'left' | 'right';
  }>([
    {
      name: 'active decisions',
      panelType: 'activeDecisions',
      defaultSize: 320,
      position: 'right',
    },
    {
      name: 'in-call chat',
      panelType: 'callChat',
      defaultSize: 380,
      position: 'right',
    },
    {
      name: 'in-call decisions',
      panelType: 'callDecisions',
      defaultSize: 380,
      position: 'right',
    },
    {
      name: 'channels list',
      panelType: 'channelsList',
      defaultSize: 240,
      position: 'left',
    },
    {
      name: 'reply thread',
      panelType: 'thread',
      defaultSize: 480,
      position: 'right',
    },
    {
      name: 'selected forum post',
      panelType: 'forumPost',
      defaultSize: 720,
      position: 'right',
    },
  ])(
    'renders the $name panel on the $position with its default width',
    ({ panelType, defaultSize, position }) => {
      render(
        <ResizablePanelLayout
          panelType={panelType}
          panel={<aside>Panel content</aside>}
          resizeHandleLabel="Resize panel"
        >
          <main>Main content</main>
        </ResizablePanelLayout>,
      );

      const panels = screen.getAllByTestId('panel');
      const resizablePanel = position === 'left' ? panels.at(0) : panels.at(-1);
      expect(resizablePanel).toHaveAttribute(
        'data-default-size',
        String(defaultSize),
      );
      expect(resizablePanel).toHaveTextContent('Panel content');
    },
  );

  it('uses the same animated resize control for panels on either side', () => {
    render(
      <ResizablePanelLayout
        panelType="channelsList"
        panel={<nav>Channels</nav>}
        resizeHandleLabel="Resize channels panel"
      >
        <main>Main content</main>
      </ResizablePanelLayout>,
    );

    expect(
      screen.getByRole('separator', { name: 'Resize channels panel' }),
    ).toHaveClass(
      'cursor-col-resize',
      'before:transition-[width]',
      'hover:before:w-0.75',
      'data-[separator=active]:before:w-0.75',
    );
  });

  it('restores the saved width for the matching panel type', () => {
    vi.mocked(localStorage.getItem).mockReturnValue(
      JSON.stringify({ thread: 612, channelsList: 312 }),
    );

    render(
      <ResizablePanelLayout
        panelType="thread"
        panel={<aside>Thread</aside>}
        resizeHandleLabel="Resize right panel"
      >
        <main>Main content</main>
      </ResizablePanelLayout>,
    );

    expect(screen.getAllByTestId('panel').at(-1)).toHaveAttribute(
      'data-default-size',
      '612',
    );
  });

  it('renders content without a resize control when the panel is absent', () => {
    render(
      <ResizablePanelLayout
        panelType="activeDecisions"
        resizeHandleLabel="Resize right panel"
      >
        <main>Main content</main>
      </ResizablePanelLayout>,
    );

    expect(screen.queryByRole('separator')).not.toBeInTheDocument();
    expect(screen.getByText('Main content')).toBeInTheDocument();
  });
});
