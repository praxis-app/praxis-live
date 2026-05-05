import { CallControls } from '@/components/calls/call-controls';
import { CallChatPanel } from '@/components/calls/call-chat-panel';
import { TopNav } from '@/components/nav/top-nav';
import {
  Drawer,
  DrawerContent,
  DrawerDescription,
  DrawerHeader,
  DrawerTitle,
} from '@/components/ui/drawer';
import { useIsDesktop } from '@/hooks/use-is-desktop';
import { useServerData } from '@/hooks/use-server-data';
import { type JoinCallRes } from '@/types/call.types';
import { type ChannelRes } from '@/types/channel.types';
import {
  ParticipantTile,
  TrackLoop,
  useParticipants,
  useTracks,
} from '@livekit/components-react';
import { Track } from 'livekit-client';
import { useTranslation } from 'react-i18next';
import { MdClose } from 'react-icons/md';
import { useState } from 'react';

interface Props {
  channel: ChannelRes;
  callConfig: JoinCallRes;
  serverName?: string;
  onLeave: () => void;
}

export const CallPanel = ({ channel, callConfig, serverName, onLeave }: Props) => {
  const participants = useParticipants();
  const isDesktop = useIsDesktop();
  const { t } = useTranslation();
  const { serverId } = useServerData();
  const [isChatOpen, setIsChatOpen] = useState(false);

  const tracks = useTracks([
    { source: Track.Source.Camera, withPlaceholder: true },
  ]);

  const tileCount = Math.max(tracks.length, 1);
  const gridColumnCount = isDesktop ? Math.ceil(Math.sqrt(tileCount)) : 1;
  const gridRowCount = Math.ceil(tileCount / gridColumnCount);

  const participantCount = t('calls.labels.participantCount', {
    count: participants.length,
  });

  const topNavSubHeader = t(
    serverName
      ? 'calls.descriptions.statusWithServer'
      : 'calls.descriptions.status',
    {
      participants: participantCount,
      serverName,
    },
  );

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
          <div
            className="grid min-h-0 flex-1 gap-3 p-3"
            style={{
              gridTemplateColumns: `repeat(${gridColumnCount}, minmax(0, 1fr))`,
              gridTemplateRows: `repeat(${gridRowCount}, minmax(0, 1fr))`,
            }}
          >
            <TrackLoop tracks={tracks}>
              <ParticipantTile className="call-participant-tile bg-muted h-full min-h-0 w-full overflow-hidden rounded-md border border-[--color-border] data-[lk-speaking=true]:border-green-500" />
            </TrackLoop>
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
          <aside className="h-full w-[380px] min-w-[320px] border-l border-[--color-border]">
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
