import { MoveProposalToForumDialog } from '@/components/polls/proposals/move-proposal-to-forum-dialog';
import { ProposalSettingsDialog } from '@/components/polls/proposals/proposal-settings-dialog';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { type ChannelRes } from '@/types/channel.types';
import { type PollRes } from '@/types/poll.types';
import { type CurrentUser } from '@/types/user.types';
import { type QueryKey } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { MdForum, MdMoreHoriz, MdSettings } from 'react-icons/md';

interface Props {
  poll: PollRes;
  channel: ChannelRes;
  feedQueryKey?: QueryKey;
  me?: CurrentUser;
  canMoveToForum?: boolean;
}

export const ProposalMenu = ({
  me,
  poll,
  channel,
  feedQueryKey,
  canMoveToForum = false,
}: Props) => {
  const [isMoveDialogOpen, setIsMoveDialogOpen] = useState(false);
  const [isSettingsDialogOpen, setIsSettingsDialogOpen] = useState(false);

  const { t } = useTranslation();

  const canMove =
    canMoveToForum &&
    !!feedQueryKey &&
    channel.channelType === 'text' &&
    poll.user.id === me?.id &&
    poll.votes.length === 0;

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="absolute top-2 right-2 z-10 size-8 text-gray-500 dark:text-gray-400"
            aria-label={t('proposals.actions.openMenu')}
          >
            <MdMoreHoriz className="size-5" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem onSelect={() => setIsSettingsDialogOpen(true)}>
            <MdSettings />
            {t('proposals.actions.viewSettings')}
          </DropdownMenuItem>
          {canMove && (
            <DropdownMenuItem onSelect={() => setIsMoveDialogOpen(true)}>
              <MdForum />
              {t('proposals.actions.moveToForum')}
            </DropdownMenuItem>
          )}
        </DropdownMenuContent>
      </DropdownMenu>

      <ProposalSettingsDialog
        open={isSettingsDialogOpen}
        onOpenChange={setIsSettingsDialogOpen}
        config={poll.config}
      />
      {canMove && feedQueryKey && (
        <MoveProposalToForumDialog
          open={isMoveDialogOpen}
          onOpenChange={setIsMoveDialogOpen}
          poll={poll}
          sourceChannel={channel}
          feedQueryKey={feedQueryKey}
        />
      )}
    </>
  );
};
