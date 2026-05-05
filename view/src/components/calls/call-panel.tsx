import { CallControls } from '@/components/calls/call-controls';
import { TopNav } from '@/components/nav/top-nav';
import { useIsDesktop } from '@/hooks/use-is-desktop';
import {
  ParticipantTile,
  TrackLoop,
  useConnectionState,
  useParticipants,
  useTracks,
} from '@livekit/components-react';
import { Track } from 'livekit-client';
import { MdClose } from 'react-icons/md';

interface Props {
  channelName: string;
  serverName?: string;
  onLeave: () => void;
}

export const CallPanel = ({ channelName, serverName, onLeave }: Props) => {
  const participants = useParticipants();
  const connectionState = useConnectionState();
  const isDesktop = useIsDesktop();
  const tracks = useTracks([
    { source: Track.Source.Camera, withPlaceholder: true },
  ]);
  const tileCount = Math.max(tracks.length, 1);
  const gridColumnCount = isDesktop ? Math.ceil(Math.sqrt(tileCount)) : 1;
  const gridRowCount = Math.ceil(tileCount / gridColumnCount);

  return (
    <div className="bg-background fixed inset-0 z-50 flex flex-col">
      <TopNav
        header={`Channel call - #${channelName}`}
        subheader={
          <>
            {serverName && `${serverName} - `}
            {participants.length} participant
            {participants.length === 1 ? '' : 's'} -{' '}
            {String(connectionState).toLowerCase()}
          </>
        }
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
            <ParticipantTile className="bg-muted h-full min-h-0 w-full overflow-hidden rounded-md border border-[--color-border]" />
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
