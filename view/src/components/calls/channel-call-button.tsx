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
import { type JoinCallRes } from '@/types/call.types';
import { Track } from 'livekit-client';
import {
  LuListChecks,
  LuMessageSquare,
  LuMic,
  LuMicOff,
  LuPhone,
  LuPhoneOff,
  LuVideo,
  LuVideoOff,
} from 'react-icons/lu';
import { toast } from 'sonner';

interface Props {
  callConfig: JoinCallRes | null;
  channelName: string;
  isJoining: boolean;
  onJoin: () => void;
  onLeave: () => void;
}

export const ChannelCallButton = ({
  callConfig,
  channelName,
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
        roomName={callConfig.roomName}
        onLeave={onLeave}
      />
    </LiveKitRoom>
  );
};

const CallPanel = ({
  channelName,
  roomName,
  onLeave,
}: {
  channelName: string;
  roomName: string;
  onLeave: () => void;
}) => {
  const participants = useParticipants();
  const connectionState = useConnectionState();
  const tracks = useTracks([
    { source: Track.Source.Camera, withPlaceholder: true },
  ]);
  const tileCount = Math.max(tracks.length, 1);
  const gridColumnCount = Math.ceil(Math.sqrt(tileCount));
  const gridRowCount = Math.ceil(tileCount / gridColumnCount);

  return (
    <div className="bg-background fixed inset-0 z-50 flex flex-col">
      <header className="flex h-14 shrink-0 items-center justify-between gap-3 border-b border-[--color-border] px-3 md:px-6">
        <div className="min-w-0">
          <div className="truncate text-sm font-semibold md:text-base">
            Channel call - #{channelName}
          </div>
          <div className="text-muted-foreground truncate text-xs md:text-sm">
            {participants.length} participant
            {participants.length === 1 ? '' : 's'} -{' '}
            {String(connectionState).toLowerCase()}
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-1">
          <Button
            aria-label="Open call chat"
            onClick={() => toast('Call chat drawer will be added later.')}
            variant="ghost"
            size="sm"
          >
            <LuMessageSquare />
            <span className="hidden sm:inline">Chat</span>
          </Button>
          <Button
            aria-label="Open CDM context"
            onClick={() => toast('Call CDM drawer will be added later.')}
            variant="ghost"
            size="sm"
          >
            <LuListChecks />
            <span className="hidden sm:inline">CDM</span>
          </Button>
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
          <div className="mx-auto flex max-w-5xl items-center justify-between gap-3">
            <div className="text-muted-foreground min-w-0 text-xs md:text-sm">
              <div className="truncate font-medium text-foreground">
                #{channelName}
              </div>
              <div className="truncate">{roomName}</div>
            </div>

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
    <div className="flex items-center gap-1">
      <Button
        aria-label={isMicrophoneEnabled ? 'Mute microphone' : 'Use microphone'}
        onClick={toggleMicrophone}
        variant={isMicrophoneEnabled ? 'secondary' : 'ghost'}
        size="icon"
      >
        {isMicrophoneEnabled ? <LuMic /> : <LuMicOff />}
      </Button>
      <Button
        aria-label={isCameraEnabled ? 'Turn camera off' : 'Use camera'}
        onClick={toggleCamera}
        variant={isCameraEnabled ? 'secondary' : 'ghost'}
        size="icon"
      >
        {isCameraEnabled ? <LuVideo /> : <LuVideoOff />}
      </Button>
      <Button
        aria-label="Leave call"
        onClick={onLeave}
        variant="ghost"
        size="icon"
      >
        <LuPhoneOff />
      </Button>
    </div>
  );
};
