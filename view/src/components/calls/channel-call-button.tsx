import { Button } from '@/components/ui/button';
import { CallPanel } from '@/components/calls/call-panel';
import { type JoinCallRes } from '@/types/call.types';
import {
  LiveKitRoom,
  RoomAudioRenderer,
} from '@livekit/components-react';
import { useTranslation } from 'react-i18next';
import { LuPhone } from 'react-icons/lu';

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

  if (!callConfig) {
    return (
      <Button
        onClick={onJoin}
        disabled={isJoining}
        variant="secondary"
        size="sm"
      >
        <LuPhone />
        {isJoining ? t('calls.actions.joining') : t('calls.actions.call')}
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
