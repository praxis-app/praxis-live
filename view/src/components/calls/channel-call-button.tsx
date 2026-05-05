import {
  LiveKitRoom,
  ParticipantTile,
  RoomAudioRenderer,
  TrackLoop,
  useConnectionState,
  useLocalParticipant,
  useParticipants,
  useTracks,
} from '@livekit/components-react';
import { Button } from '@/components/ui/button';
import { useIsDesktop } from '@/hooks/use-is-desktop';
import { type JoinCallRes } from '@/types/call.types';
import { Track } from 'livekit-client';
import {
  LuMessageSquare,
  LuMic,
  LuMicOff,
  LuMinimize2,
  LuPhone,
  LuPhoneOff,
  LuVideo,
  LuVideoOff,
} from 'react-icons/lu';
import { toast } from 'sonner';

interface Props {
  callConfig: JoinCallRes | null;
  channelName: string;
  serverName?: string;
  isJoining: boolean;
  onJoin: () => void;
  onLeave: () => void;
}

export const ChannelCallButton = ({
  callConfig,
  channelName,
  serverName,
  isJoining,
  onJoin,
  onLeave,
}: Props) => {
  if (!callConfig) {
    return (
      <Button
        onClick={onJoin}
        disabled={isJoining}
        variant="secondary"
        size="sm"
      >
        <LuPhone />
        {isJoining ? 'Joining' : 'Call'}
      </Button>
    );
  }

  return (
    <LiveKitRoom
      serverUrl={callConfig.livekitUrl}
      token={callConfig.token}
      connect
      audio={false}
      video={false}
      onDisconnected={onLeave}
    >
      <RoomAudioRenderer />
      <CallPanel
        channelName={channelName}
        serverName={serverName}
        onLeave={onLeave}
      />
    </LiveKitRoom>
  );
};

const CallPanel = ({
  channelName,
  serverName,
  onLeave,
}: {
  channelName: string;
  serverName?: string;
  onLeave: () => void;
}) => {
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
      <header className="flex h-14 shrink-0 items-center justify-between gap-3 border-b border-[--color-border] px-3 md:px-6">
        <div className="min-w-0">
          <div className="truncate text-sm font-semibold md:text-base">
            Channel call - #{channelName}
          </div>
          <div className="text-muted-foreground truncate text-xs md:text-sm">
            {serverName && `${serverName} - `}
            {participants.length} participant
            {participants.length === 1 ? '' : 's'} -{' '}
            {String(connectionState).toLowerCase()}
          </div>
        </div>

      </header>

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

const CallControls = ({ onLeave }: { onLeave: () => void }) => {
  const { localParticipant, isMicrophoneEnabled, isCameraEnabled } =
    useLocalParticipant();

  const toggleMicrophone = async () => {
    try {
      await localParticipant.setMicrophoneEnabled(!isMicrophoneEnabled);
    } catch {
      toast('Unable to use the microphone.');
    }
  };

  const toggleCamera = async () => {
    try {
      await localParticipant.setCameraEnabled(!isCameraEnabled);
    } catch {
      toast('Unable to use the camera.');
    }
  };

  return (
    <div className="flex items-center justify-center gap-2">
      <Button
        aria-label={isMicrophoneEnabled ? 'Mute microphone' : 'Use microphone'}
        onClick={toggleMicrophone}
        variant={isMicrophoneEnabled ? 'secondary' : 'ghost'}
        size="icon"
        className="size-11 rounded-full"
      >
        {isMicrophoneEnabled ? <LuMic /> : <LuMicOff />}
      </Button>
      <Button
        aria-label={isCameraEnabled ? 'Turn camera off' : 'Use camera'}
        onClick={toggleCamera}
        variant={isCameraEnabled ? 'secondary' : 'ghost'}
        size="icon"
        className="size-11 rounded-full"
      >
        {isCameraEnabled ? <LuVideo /> : <LuVideoOff />}
      </Button>
      <Button
        aria-label="Open channel chat"
        onClick={() => undefined}
        variant="ghost"
        size="icon"
        className="size-11 rounded-full"
      >
        <LuMessageSquare />
      </Button>
      <Button
        aria-label="Minimize call"
        onClick={() => undefined}
        variant="ghost"
        size="icon"
        className="size-11 rounded-full"
      >
        <LuMinimize2 />
      </Button>
      <Button
        aria-label="Leave call"
        onClick={onLeave}
        variant="destructive"
        size="icon"
        className="size-11 rounded-full"
      >
        <LuPhoneOff />
      </Button>
    </div>
  );
};
