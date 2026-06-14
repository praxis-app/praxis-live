import { CallPanel } from '@/components/calls/call-panel/call-panel';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { type JoinCallRes } from '@/types/call.types';
import { type ChannelRes } from '@/types/channel.types';
import { LiveKitRoom, RoomAudioRenderer } from '@livekit/components-react';
import { useTranslation } from 'react-i18next';
import { TbVideo } from 'react-icons/tb';
import { toast } from 'sonner';

interface Props {
  callConfig: JoinCallRes | null;
  channel: ChannelRes;
  serverName?: string;
  isJoining: boolean;
  onJoin: () => void;
  onLeave: () => void | Promise<void>;
}

export const ChannelCallButton = ({
  callConfig,
  channel,
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
              onClick={() => onJoin()}
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
      onError={() => {
        toast(t('calls.errors.unavailable'), { id: 'calls-unavailable' });
        void onLeave();
      }}
      onDisconnected={onLeave}
    >
      <RoomAudioRenderer />
      <CallPanel
        channel={channel}
        callConfig={callConfig}
        serverName={serverName}
        onLeave={onLeave}
      />
    </LiveKitRoom>
  );
};
