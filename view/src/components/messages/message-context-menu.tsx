import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from '@/components/ui/context-menu';
import { useDeferredMenuAction } from '@/hooks/use-deferred-menu-action';
import { type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { LuReply } from 'react-icons/lu';
import { MdLink } from 'react-icons/md';

interface Props {
  children: ReactNode;
  onOpenThread: () => void;
  onCopyThreadLink: () => void;
}

export const MessageContextMenu = ({
  children,
  onOpenThread,
  onCopyThreadLink,
}: Props) => {
  const { deferUntilClosed, runPendingAction } = useDeferredMenuAction();

  const { t } = useTranslation();

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
      <ContextMenuContent onCloseAutoFocus={runPendingAction}>
        <ContextMenuItem onSelect={deferUntilClosed(onOpenThread)}>
          <LuReply />
          {t('messages.actions.reply')}
        </ContextMenuItem>
        <ContextMenuItem onSelect={onCopyThreadLink}>
          <MdLink />
          {t('messages.actions.copyLink')}
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
};
