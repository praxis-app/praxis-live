import { Button } from '@/components/ui/button';
import { cn } from '@/lib/shared.utils';
import { useLocalParticipant } from '@livekit/components-react';
import {
  LuMessageSquare,
  LuMic,
  LuMicOff,
  LuMinimize2,
  LuPhoneOff,
  LuVideo,
  LuVideoOff,
} from 'react-icons/lu';
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
        aria-label={isCameraEnabled ? 'Turn camera off' : 'Use camera'}
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
        aria-label="Open channel chat"
        onClick={() => undefined}
        variant="ghost"
        size="icon"
        className={controlButtonClassName}
      >
        <LuMessageSquare />
      </Button>
      <Button
        aria-label="Minimize call"
        onClick={() => undefined}
        variant="ghost"
        size="icon"
        className={controlButtonClassName}
      >
        <LuMinimize2 />
      </Button>
      <Button
        aria-label="Leave call"
        onClick={onLeave}
        variant="ghost"
        size="icon"
        className="bg-destructive text-white hover:bg-destructive/85 focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:hover:bg-destructive/50 size-11 rounded-full"
      >
        <LuPhoneOff />
      </Button>
    </div>
  );
};
