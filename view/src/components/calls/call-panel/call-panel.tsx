import { CallChatPanel } from '@/components/calls/call-chat-panel';
import { CallControls } from '@/components/calls/call-controls';
import { TopNav } from '@/components/nav/top-nav';
import {
  Drawer,
  DrawerContent,
  DrawerDescription,
  DrawerHeader,
  DrawerTitle,
} from '@/components/ui/drawer';
import { BrowserEvents, KeyCodes } from '@/constants/shared.constants';
import { useIsDesktop } from '@/hooks/use-is-desktop';
import { useServerData } from '@/hooks/use-server-data';
import { cn } from '@/lib/shared.utils';
import { type JoinCallRes } from '@/types/call.types';
import { type ChannelRes } from '@/types/channel.types';
import {
  GridLayout,
  useParticipants,
  useRoomContext,
  useTracks,
} from '@livekit/components-react';
import { Track } from 'livekit-client';
import { type CSSProperties, useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { MdClose } from 'react-icons/md';
import { CallParticipantTile } from './call-participant-tile';

const getGridColumnCount = (
  trackCount: number,
  isDesktop: boolean,
  isChatOpen: boolean,
) => {
  if (trackCount <= 1) {
    return 1;
  }
  if (trackCount === 2 && (!isDesktop || isChatOpen)) {
    return 1;
  }
  if (trackCount <= 4) {
    return 2;
  }
  if (trackCount <= 9) {
    return 3;
  }
  if (trackCount <= 16) {
    return 4;
  }

  return 5;
};

interface Props {
  channel: ChannelRes;
  callConfig: JoinCallRes;
  serverName?: string;
  onLeave: () => void | Promise<void>;
}

export const CallPanel = ({
  channel,
  callConfig,
  serverName,
  onLeave,
}: Props) => {
  const [isChatOpen, setIsChatOpen] = useState(false);
  const { serverId } = useServerData();

  const participants = useParticipants();
  const room = useRoomContext();

  const tracks = useTracks([
    {
      source: Track.Source.Camera,
      withPlaceholder: true,
    },
  ]);

  const isDesktop = useIsDesktop();
  const { t } = useTranslation();

  const handleLeave = useCallback(async () => {
    await room.disconnect();
    await onLeave();
  }, [onLeave, room]);

  const handleEscapeKey = useCallback(() => {
    if (isChatOpen) {
      setIsChatOpen(false);
      return;
    }
    void handleLeave();
  }, [handleLeave, isChatOpen]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === KeyCodes.Escape) {
        handleEscapeKey();
      }
    };

    window.addEventListener(BrowserEvents.Keydown, handleKeyDown);
    return () => {
      window.removeEventListener(BrowserEvents.Keydown, handleKeyDown);
    };
  }, [handleEscapeKey]);

  const tileLayoutKey = `${isDesktop ? 'desktop' : 'mobile'}-${isChatOpen ? 'chat-open' : 'chat-closed'}-${tracks.length}`;

  const topNavSubHeader = t(
    serverName
      ? 'calls.descriptions.statusWithServer'
      : 'calls.descriptions.status',
    {
      participants: t('calls.labels.participantCount', {
        count: participants.length,
      }),
      serverName,
    },
  );

  const gridColumnCount = getGridColumnCount(
    tracks.length,
    isDesktop,
    isChatOpen,
  );

  const shouldStackTiles =
    tracks.length === 2 && (!isDesktop || (isDesktop && isChatOpen));

  const gridLayoutStyle: CSSProperties = {
    ...(shouldStackTiles && {
      gridTemplateColumns: 'repeat(1, minmax(0, 1fr))',
      gridTemplateRows: 'repeat(2, minmax(0, 1fr))',
    }),
  };

  return (
    <div className="bg-background fixed inset-0 z-50 flex flex-col">
      <TopNav
        header={t('calls.headers.channelCall', { channelName: channel.name })}
        subheader={topNavSubHeader}
        onBackClick={handleLeave}
        backBtnIcon={<MdClose className="size-6" />}
        showSearch={false}
      />

      <main className="flex min-h-0 flex-1">
        <div className="flex min-w-0 flex-1 flex-col">
          <div className="min-h-0 flex-1 overflow-hidden p-3">
            <div className="flex h-full min-h-0 w-full flex-col items-center justify-center overflow-hidden">
              <GridLayout
                tracks={tracks}
                className={cn(
                  'h-full min-h-0 w-full grid-rows-[repeat(var(--lk-row-count),minmax(0,1fr))] p-0 [--lk-grid-gap:0.75rem]',
                  shouldStackTiles &&
                    '[&>*:first-child]:items-end [&>*:last-child]:items-start',
                  shouldStackTiles && !isDesktop && '[--lk-grid-gap:0.375rem]',
                  gridColumnCount === 2 &&
                    '[&>*:nth-child(even)]:justify-start [&>*:nth-child(odd)]:justify-end',
                  gridColumnCount === 3 &&
                    '[&>*:nth-child(3n)]:justify-start [&>*:nth-child(3n+1)]:justify-end [&>*:nth-child(3n+2)]:justify-center',
                  gridColumnCount === 4 &&
                    '[&>*:nth-child(4n)]:justify-start [&>*:nth-child(4n+1)]:justify-end [&>*:nth-child(4n+2)]:justify-end [&>*:nth-child(4n+3)]:justify-start',
                  gridColumnCount === 5 &&
                    '[&>*:nth-child(5n)]:justify-start [&>*:nth-child(5n+1)]:justify-end [&>*:nth-child(5n+2)]:justify-end [&>*:nth-child(5n+3)]:justify-center [&>*:nth-child(5n+4)]:justify-start',
                )}
                style={gridLayoutStyle}
              >
                <CallParticipantTile layoutKey={tileLayoutKey} />
              </GridLayout>
            </div>
          </div>

          <div className="border-t border-[--color-border] px-3 py-3">
            <div className="flex items-center justify-center">
              <CallControls
                onLeave={handleLeave}
                onOpenChat={() => setIsChatOpen((open) => !open)}
              />
            </div>
          </div>
        </div>

        {isDesktop && isChatOpen && (
          <aside className="h-full w-[380px] min-w-0 border-l border-[--color-border]">
            <CallChatPanel
              serverId={serverId}
              channel={channel}
              callId={callConfig.call.id}
            />
          </aside>
        )}
      </main>

      {!isDesktop && (
        <Drawer open={isChatOpen} onOpenChange={setIsChatOpen}>
          <DrawerContent className="h-[86vh]">
            <DrawerHeader className="sr-only">
              <DrawerTitle>{t('calls.headers.inCallChat')}</DrawerTitle>
              <DrawerDescription>
                {t('calls.descriptions.inCallChat')}
              </DrawerDescription>
            </DrawerHeader>
            <CallChatPanel
              serverId={serverId}
              channel={channel}
              callId={callConfig.call.id}
            />
          </DrawerContent>
        </Drawer>
      )}
    </div>
  );
};
