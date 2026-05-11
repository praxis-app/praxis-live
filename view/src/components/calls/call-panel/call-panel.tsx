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
import { type JoinCallRes } from '@/types/call.types';
import { type ChannelRes } from '@/types/channel.types';
import {
  GridLayout,
  useParticipants,
  useTracks,
} from '@livekit/components-react';
import { Track } from 'livekit-client';
import { type CSSProperties, useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { MdClose } from 'react-icons/md';
import { CallParticipantTile } from './call-participant-tile';

interface Props {
  channel: ChannelRes;
  callConfig: JoinCallRes;
  serverName?: string;
  onLeave: () => void;
}

export const CallPanel = ({
  channel,
  callConfig,
  serverName,
  onLeave,
}: Props) => {
  const [isChatOpen, setIsChatOpen] = useState(false);
  const participants = useParticipants();
  const { serverId } = useServerData();

  const isDesktop = useIsDesktop();
  const { t } = useTranslation();

  const tracks = useTracks([
    { source: Track.Source.Camera, withPlaceholder: true },
  ]);

  const handleEscapeKey = useCallback(() => {
    if (isChatOpen) {
      setIsChatOpen(false);
      return;
    }

    onLeave();
  }, [isChatOpen, onLeave]);

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

  const shouldStackTiles =
    tracks.length === 2 && (!isDesktop || (isDesktop && isChatOpen));

  const gridLayoutStyle: CSSProperties & Record<'--lk-grid-gap', string> = {
    ...(shouldStackTiles && {
      gridTemplateColumns: 'repeat(1, minmax(0, 1fr))',
      gridTemplateRows: 'repeat(2, minmax(0, 1fr))',
    }),
    '--lk-grid-gap': '0.75rem',
  };

  return (
    <div className="bg-background fixed inset-0 z-50 flex flex-col">
      <TopNav
        header={t('calls.headers.channelCall', { channelName: channel.name })}
        subheader={topNavSubHeader}
        onBackClick={onLeave}
        backBtnIcon={<MdClose className="size-6" />}
        showSearch={false}
      />

      <main className="flex min-h-0 flex-1">
        <div className="flex min-w-0 flex-1 flex-col">
          <div className="min-h-0 flex-1 overflow-hidden p-3">
            <div className="call-grid-layout-wrapper h-full min-h-0">
              <GridLayout
                tracks={tracks}
                className="call-grid-layout h-full min-h-0 w-full p-0"
                style={gridLayoutStyle}
              >
                <CallParticipantTile layoutKey={tileLayoutKey} />
              </GridLayout>
            </div>
          </div>

          <div className="border-t border-[--color-border] px-3 py-3">
            <div className="flex items-center justify-center">
              <CallControls
                onLeave={onLeave}
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
