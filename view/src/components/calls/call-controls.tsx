import { Button } from '@/components/ui/button';
import { cn } from '@/lib/shared.utils';
import { useLocalParticipant } from '@livekit/components-react';
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

const controlButtonClassName =
  'size-11 rounded-full bg-secondary text-secondary-foreground/85 hover:bg-secondary/70';

const activeControlButtonClassName =
  'bg-primary text-primary-foreground hover:bg-primary/90';

export const CallControls = ({ onLeave }: Props) => {
  const { localParticipant, isMicrophoneEnabled, isCameraEnabled } =
    useLocalParticipant();
  const { t } = useTranslation();

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
    <div className="flex items-center justify-center gap-2">
      <Button
        aria-label={
          isMicrophoneEnabled
            ? t('calls.labels.muteMicrophone')
            : t('calls.labels.useMicrophone')
        }
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
      <Button
        aria-label={
          isCameraEnabled
            ? t('calls.labels.turnCameraOff')
            : t('calls.labels.useCamera')
        }
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
      <Button
        aria-label={t('calls.labels.minimizeCall')}
        onClick={() => undefined}
        variant="ghost"
        size="icon"
        className={controlButtonClassName}
      >
        <LuMinimize2 />
      </Button>
      <Button
        aria-label={t('calls.labels.openChannelChat')}
        onClick={() => undefined}
        variant="ghost"
        size="icon"
        className={controlButtonClassName}
      >
        <LuMessageSquare />
      </Button>
      <Button
        aria-label={t('calls.labels.leaveCall')}
        onClick={onLeave}
        variant="ghost"
        size="icon"
        className="bg-destructive hover:bg-destructive/85 focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:hover:bg-destructive/50 size-11 rounded-full text-white"
      >
        <MdOutlineCallEnd />
      </Button>
    </div>
  );
};
