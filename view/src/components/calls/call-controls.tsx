import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/shared.utils';
import { useLocalParticipant } from '@livekit/components-react';
import { type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import {
  LuMessageSquare,
  LuMic,
  LuMicOff,
  LuMinimize2,
  LuVideo,
  LuVideoOff,
} from 'react-icons/lu';
import { MdOutlineCallEnd } from 'react-icons/md';
import { toast } from 'sonner';

interface Props {
  onLeave: () => void;
}

interface CallControlTooltipProps {
  children: ReactNode;
  label: string;
}

const controlButtonClassName =
  'size-11 rounded-full bg-secondary text-secondary-foreground/85 hover:bg-secondary/70';

const activeControlButtonClassName =
  'bg-primary text-primary-foreground hover:bg-primary/90';

const CallControlTooltip = ({ children, label }: CallControlTooltipProps) => (
  <Tooltip>
    <TooltipTrigger asChild>{children}</TooltipTrigger>
    <TooltipContent>{label}</TooltipContent>
  </Tooltip>
);

export const CallControls = ({ onLeave }: Props) => {
  const { localParticipant, isMicrophoneEnabled, isCameraEnabled } =
    useLocalParticipant();

  const { t } = useTranslation();

  const microphoneLabel = isMicrophoneEnabled
    ? t('calls.labels.muteMicrophone')
    : t('calls.labels.useMicrophone');
  const cameraLabel = isCameraEnabled
    ? t('calls.labels.turnCameraOff')
    : t('calls.labels.useCamera');

  const minimizeCallLabel = t('calls.labels.minimizeCall');
  const openChannelChatLabel = t('calls.labels.openChannelChat');
  const leaveCallLabel = t('calls.labels.leaveCall');

  const toggleMicrophone = async () => {
    try {
      await localParticipant.setMicrophoneEnabled(!isMicrophoneEnabled);
    } catch {
      toast(t('calls.errors.microphoneUnavailable'));
    }
  };

  const toggleCamera = async () => {
    try {
      await localParticipant.setCameraEnabled(!isCameraEnabled);
    } catch {
      toast(t('calls.errors.cameraUnavailable'));
    }
  };

  return (
    <TooltipProvider>
      <div className="flex items-center justify-center gap-2">
        <CallControlTooltip label={microphoneLabel}>
          <Button
            aria-label={microphoneLabel}
            onClick={toggleMicrophone}
            variant="ghost"
            size="icon"
            className={cn(
              controlButtonClassName,
              isMicrophoneEnabled && activeControlButtonClassName,
            )}
          >
            {isMicrophoneEnabled ? <LuMic /> : <LuMicOff />}
          </Button>
        </CallControlTooltip>

        <CallControlTooltip label={cameraLabel}>
          <Button
            aria-label={cameraLabel}
            onClick={toggleCamera}
            variant="ghost"
            size="icon"
            className={cn(
              controlButtonClassName,
              isCameraEnabled && activeControlButtonClassName,
            )}
          >
            {isCameraEnabled ? <LuVideo /> : <LuVideoOff />}
          </Button>
        </CallControlTooltip>

        <CallControlTooltip label={minimizeCallLabel}>
          <Button
            aria-label={minimizeCallLabel}
            onClick={() => undefined}
            variant="ghost"
            size="icon"
            className={controlButtonClassName}
          >
            <LuMinimize2 />
          </Button>
        </CallControlTooltip>

        <CallControlTooltip label={openChannelChatLabel}>
          <Button
            aria-label={openChannelChatLabel}
            onClick={() => undefined}
            variant="ghost"
            size="icon"
            className={controlButtonClassName}
          >
            <LuMessageSquare />
          </Button>
        </CallControlTooltip>

        <CallControlTooltip label={leaveCallLabel}>
          <Button
            aria-label={leaveCallLabel}
            onClick={onLeave}
            variant="ghost"
            size="icon"
            className="bg-destructive hover:bg-destructive/85 focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:hover:bg-destructive/50 size-11 rounded-full text-white"
          >
            <MdOutlineCallEnd />
          </Button>
        </CallControlTooltip>
      </div>
    </TooltipProvider>
  );
};
