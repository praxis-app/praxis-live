import { Button } from '@/components/ui/button';
import { cn } from '@/lib/shared.utils';
import { useTranslation } from 'react-i18next';
import { LuReply } from 'react-icons/lu';

interface Props {
  onReply: () => void;
  className?: string;
}

export const MessageReplyButton = ({ onReply, className }: Props) => {
  const { t } = useTranslation();

  return (
    <Button
      type="button"
      variant="outline"
      size="icon"
      aria-label={t('messages.actions.reply')}
      className={cn(
        'bg-background/95 absolute -top-1 right-0 z-10 hidden size-8 opacity-0 shadow-sm transition-opacity group-hover/message:opacity-100 focus-visible:opacity-100 motion-reduce:transition-none md:inline-flex',
        className,
      )}
      onClick={onReply}
    >
      <LuReply className="text-muted-foreground size-4.5" />
    </Button>
  );
};
