import { CallControls } from '@/components/calls/call-controls';
import { TopNav } from '@/components/nav/top-nav';
import { useIsDesktop } from '@/hooks/use-is-desktop';
import {
  ParticipantTile,
  TrackLoop,
  useParticipants,
  useTracks,
} from '@livekit/components-react';
import { Track } from 'livekit-client';
import { useTranslation } from 'react-i18next';
import { MdClose } from 'react-icons/md';

interface Props {
  channelName: string;
  serverName?: string;
  onLeave: () => void;
}

export const CallPanel = ({ channelName, serverName, onLeave }: Props) => {
  const participants = useParticipants();
  const isDesktop = useIsDesktop();
  const { t } = useTranslation();

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
        header={t('calls.headers.channelCall', { channelName })}
        subheader={topNavSubHeader}
        onBackClick={onLeave}
        backBtnIcon={<MdClose className="size-6" />}
        showSearch={false}
      />

      <main className="flex min-h-0 flex-1 flex-col">
        <div
          className="grid min-h-0 flex-1 gap-3 p-3"
          style={{
            gridTemplateColumns: `repeat(${gridColumnCount}, minmax(0, 1fr))`,
            gridTemplateRows: `repeat(${gridRowCount}, minmax(0, 1fr))`,
          }}
        >
          <TrackLoop tracks={tracks}>
            <ParticipantTile className="bg-muted h-full min-h-0 w-full overflow-hidden rounded-md border border-[--color-border] data-[lk-speaking=true]:border-green-500" />
          </TrackLoop>
        </div>

        <div className="border-t border-[--color-border] px-3 py-3">
          <div className="flex items-center justify-center">
            <CallControls onLeave={onLeave} />
          </div>
        </div>
      </main>
    </div>
  );
};
