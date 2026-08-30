import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/shared.utils';
import { useTranslation } from 'react-i18next';

interface Props {
  /** Upload completion between 0 and 1, or undefined while indeterminate. */
  progress?: number;
  className?: string;
}

/**
 * Covers an attached image while it uploads. The bar tracks bytes sent, then
 * pulses once the server takes over to process the image.
 */
export const ImageUploadOverlay = ({ progress, className }: Props) => {
  const { t } = useTranslation();

  const clamped =
    progress === undefined ? undefined : Math.min(Math.max(progress, 0), 1);
  const isProcessing = clamped === undefined || clamped >= 1;
  const percent = Math.round((clamped ?? 0) * 100);

  return (
    <div
      data-testid="image-upload-overlay"
      className={cn(
        'bg-background/80 absolute inset-0 flex items-center justify-center',
        'rounded-md px-4 backdrop-blur-xs',
        className,
      )}
    >
      <Progress
        aria-label={t('images.labels.uploadingImage')}
        // `Progress` does not forward its value to the underlying element.
        aria-valuenow={isProcessing ? undefined : percent}
        value={isProcessing ? 100 : percent}
        className={cn('h-1.5 w-full max-w-32', isProcessing && 'animate-pulse')}
      />
    </div>
  );
};
