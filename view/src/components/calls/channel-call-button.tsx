import { CallPanel } from '@/components/calls/call-panel';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { type JoinCallRes } from '@/types/call.types';
import { LiveKitRoom, RoomAudioRenderer } from '@livekit/components-react';
import { useTranslation } from 'react-i18next';
import { TbVideo } from 'react-icons/tb';

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
  const { t } = useTranslation();
  const callLabel = t('calls.actions.call');

  if (!callConfig) {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              aria-label={callLabel}
              onClick={onJoin}
              disabled={isJoining}
              variant="ghost"
              size="icon"
            >
              <TbVideo className="size-6" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{callLabel}</TooltipContent>
        </Tooltip>
      </TooltipProvider>
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
