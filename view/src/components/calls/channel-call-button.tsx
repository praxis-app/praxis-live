import {
  LiveKitRoom,
  RoomAudioRenderer,
  useConnectionState,
  useParticipants,
} from '@livekit/components-react';
import { Button } from '@/components/ui/button';
import { type JoinCallRes } from '@/types/call.types';
import { LuPhone, LuPhoneOff } from 'react-icons/lu';

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
        <CallPopover roomName={callConfig.roomName} />
      </div>
    </LiveKitRoom>
  );
};

const CallPopover = ({ roomName }: { roomName: string }) => {
  const participants = useParticipants();
  const connectionState = useConnectionState();

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

      <div className="grid grid-cols-2 gap-2">
        {participants.map((participant) => (
          <div
            key={participant.identity}
            className="bg-muted flex aspect-video min-w-0 items-center justify-center rounded-md border border-[--color-border] px-2 text-center"
          >
            <span className="truncate text-sm font-medium">
              {participant.name || participant.identity}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
};
