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
  isJoining: boolean;
  onJoin: () => void;
  onLeave: () => void;
}

export const ChannelCallButton = ({
  callConfig,
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
      <div className="relative">
        <Button onClick={onLeave} variant="secondary" size="sm">
          <LuPhoneOff />
          Leave
        </Button>
        <RoomAudioRenderer />
        <CallPanel roomName={callConfig.roomName} onLeave={onLeave} />
      </div>
    </LiveKitRoom>
  );
};

const CallPanel = ({
  roomName,
  onLeave,
}: {
  roomName: string;
  onLeave: () => void;
}) => {
  const participants = useParticipants();
  const connectionState = useConnectionState();
  const tracks = useTracks([
    { source: Track.Source.Camera, withPlaceholder: true },
  ]);

  return (
    <div className="bg-background absolute top-11 right-0 z-30 w-[min(22rem,calc(100vw-1rem))] rounded-md border border-[--color-border] p-3 shadow-lg">
      <div className="mb-3 flex items-center justify-between gap-3">
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">Channel call</div>
          <div className="text-muted-foreground truncate text-xs">
            {roomName}
          </div>
        </div>
        <div className="text-muted-foreground text-xs capitalize">
          {String(connectionState).toLowerCase()}
        </div>
      </div>

      <div className="mb-3 grid max-h-[18rem] grid-cols-2 gap-2 overflow-y-auto">
        <TrackLoop tracks={tracks}>
          <ParticipantTile className="bg-muted aspect-video overflow-hidden rounded-md border border-[--color-border]" />
        </TrackLoop>
      </div>

      <div className="flex items-center justify-between gap-2">
        <div className="text-muted-foreground text-xs">
          {participants.length} participant
          {participants.length === 1 ? '' : 's'}
        </div>
        <CallControls onLeave={onLeave} />
      </div>
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
