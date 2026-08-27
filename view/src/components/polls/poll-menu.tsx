import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { LuReply } from 'react-icons/lu';
import { MdMoreHoriz } from 'react-icons/md';

interface Props {
  onOpenThread: () => void;
}

export const PollMenu = ({ onOpenThread }: Props) => {
  const pendingActionRef = useRef<(() => void) | null>(null);

  const { t } = useTranslation();

  // Defer menu actions until the close animation ends to avoid a visible flicker
  const deferUntilClosed = (action: () => void) => () => {
    pendingActionRef.current = action;
  };

  const runPendingAction = () => {
    const action = pendingActionRef.current;
    pendingActionRef.current = null;
    action?.();
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="absolute top-2 right-2 z-10 size-8 text-gray-500 dark:text-gray-400"
          aria-label={t('polls.actions.openMenu')}
        >
          <MdMoreHoriz className="size-5" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" onCloseAutoFocus={runPendingAction}>
        <DropdownMenuItem onSelect={deferUntilClosed(onOpenThread)}>
          <LuReply />
          {t('messages.actions.reply')}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
};
