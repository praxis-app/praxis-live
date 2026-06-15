import { CallPanel } from '@/components/calls/call-panel/call-panel';
import { PreJoinScreen } from '@/components/calls/pre-join-screen';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import {
  type CallJoinPreferences,
  type JoinCallRes,
} from '@/types/call.types';
import { type ChannelRes } from '@/types/channel.types';
import {
  LiveKitRoom,
  RoomAudioRenderer,
  useLocalParticipant,
} from '@livekit/components-react';
import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { TbVideo } from 'react-icons/tb';
import { toast } from 'sonner';

interface Props {
  callConfig: JoinCallRes | null;
  channel: ChannelRes;
  callPreferences: CallJoinPreferences | null;
  serverName?: string;
  isJoining: boolean;
  isPreJoinOpen: boolean;
  onCancelPreJoin: () => void;
  onConfirmJoin: (preferences: CallJoinPreferences) => void;
  onJoin: () => void;
  onLeave: () => void | Promise<void>;
}

interface ApplyCallPreferencesProps {
  preferences: CallJoinPreferences | null;
}

const ApplyCallPreferences = ({ preferences }: ApplyCallPreferencesProps) => {
  const hasAppliedRef = useRef(false);
  const { localParticipant } = useLocalParticipant();
  const { t } = useTranslation();

  useEffect(() => {
    if (!preferences || hasAppliedRef.current) {
      return;
    }

    hasAppliedRef.current = true;

    const applyPreferences = async () => {
      try {
        await localParticipant.setMicrophoneEnabled(
          preferences.audioEnabled,
          preferences.audioDeviceId === 'default'
            ? undefined
            : { deviceId: preferences.audioDeviceId },
        );
      } catch {
        toast(t('calls.errors.microphoneUnavailable'));
      }

      try {
        await localParticipant.setCameraEnabled(
          preferences.videoEnabled,
          preferences.videoDeviceId === 'default'
            ? undefined
            : { deviceId: preferences.videoDeviceId },
        );
      } catch {
        toast(t('calls.errors.cameraUnavailable'));
      }
    };

    void applyPreferences();
  }, [localParticipant, preferences, t]);

  return null;
};

export const ChannelCallButton = ({
  callConfig,
  callPreferences,
  channel,
  serverName,
  isJoining,
  isPreJoinOpen,
  onCancelPreJoin,
  onConfirmJoin,
  onJoin,
  onLeave,
}: Props) => {
  const { t } = useTranslation();
  const callLabel = t('calls.actions.call');

  if (!callConfig) {
    return (
      <>
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

        {isPreJoinOpen && (
          <PreJoinScreen
            channel={channel}
            isJoining={isJoining}
            onCancel={onCancelPreJoin}
            onJoin={onConfirmJoin}
            serverName={serverName}
          />
        )}
      </>
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
      <ApplyCallPreferences preferences={callPreferences} />
      <CallPanel
        channel={channel}
        callConfig={callConfig}
        serverName={serverName}
        onLeave={onLeave}
      />
    </LiveKitRoom>
  );
};
