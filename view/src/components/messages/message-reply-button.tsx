import { Button } from '@/components/ui/button';
import { useTranslation } from 'react-i18next';
import { LuMessageSquareText } from 'react-icons/lu';

interface Props {
  onReply: () => void;
}

export const MessageReplyButton = ({ onReply }: Props) => {
  const { t } = useTranslation();

  return (
    <Button
      type="button"
      variant="outline"
      size="icon"
      aria-label={t('messages.actions.reply')}
      className="bg-background/95 absolute -top-1 right-0 z-10 hidden size-8 opacity-0 shadow-sm transition-opacity group-hover/message:opacity-100 focus-visible:opacity-100 motion-reduce:transition-none md:inline-flex"
      onClick={onReply}
    >
      <LuMessageSquareText className="text-muted-foreground size-4.5" />
    </Button>
  );
};
