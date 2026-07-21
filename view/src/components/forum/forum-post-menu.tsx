import { api } from '@/client/api-client';
import { CreateProposalForm } from '@/components/polls/proposals/create-proposal-form/create-proposal-form';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Separator } from '@/components/ui/separator';
import { useServerData } from '@/hooks/use-server-data';
import { handleError } from '@/lib/error.utils';
import { type ChannelRes } from '@/types/channel.types';
import { type ForumPostRes } from '@/types/forum.types';
import { type CreatePollReq } from '@/types/poll.types';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { LuListTodo } from 'react-icons/lu';
import { MdLockOutline, MdMoreHoriz, MdSettings } from 'react-icons/md';

interface Props {
  channel: ChannelRes;
  post: ForumPostRes;
  isAuthor: boolean;
  onViewProposalSettings: () => void;
}

export const ForumPostMenu = ({
  channel,
  post,
  isAuthor,
  onViewProposalSettings,
}: Props) => {
  const { t } = useTranslation();
  const { serverId } = useServerData();
  const queryClient = useQueryClient();
  const dialogRef = useRef<HTMLDivElement>(null);
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [isMenuOpen, setIsMenuOpen] = useState(false);

  const invalidateForum = () =>
    queryClient.invalidateQueries({
      queryKey: ['servers', serverId, 'channels', channel.id, 'forum'],
    });

  const { mutate: closePost, isPending: isClosing } = useMutation({
    mutationFn: () => {
      if (!serverId) throw new Error('Server ID is required');
      return api.closeForumPost(serverId, channel.id, post.id);
    },
    onSuccess: () => void invalidateForum(),
    onError: handleError,
  });

  const createProposal = async (request: CreatePollReq) => {
    if (!serverId) throw new Error('Server ID is required');
    const { post: updatedPost } = await api.createForumPostProposal(
      serverId,
      channel.id,
      post.id,
      request,
    );
    if (!updatedPost.proposal) {
      throw new Error('Created forum proposal is missing');
    }
    void invalidateForum();
    return { poll: updatedPost.proposal };
  };

  return (
    <>
      <DropdownMenu open={isMenuOpen} onOpenChange={setIsMenuOpen}>
        <DropdownMenuTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-8 shrink-0"
            aria-label={t('forums.actions.openPostMenu')}
            onPointerDown={(event) => event.preventDefault()}
            onPointerUp={() => setIsMenuOpen((open) => !open)}
          >
            <MdMoreHoriz className="size-5" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          {post.proposal && (
            <DropdownMenuItem onSelect={onViewProposalSettings}>
              <MdSettings />
              {t('forums.actions.viewProposalSettings')}
            </DropdownMenuItem>
          )}
          {isAuthor && !post.proposal && (
            <DropdownMenuItem onSelect={() => setIsCreateOpen(true)}>
              <LuListTodo />
              {t('forums.actions.createProposalFromDiscussion')}
            </DropdownMenuItem>
          )}
          {isAuthor && post.status === 'open' && (
            <DropdownMenuItem disabled={isClosing} onSelect={() => closePost()}>
              <MdLockOutline />
              {t('forums.actions.closePost')}
            </DropdownMenuItem>
          )}
        </DropdownMenuContent>
      </DropdownMenu>

      <Dialog open={isCreateOpen} onOpenChange={setIsCreateOpen}>
        <DialogContent
          ref={dialogRef}
          className="max-h-[90vh] overflow-y-auto md:w-xl"
        >
          <DialogHeader>
            <DialogTitle>
              {t('forums.actions.createProposalFromDiscussion')}
            </DialogTitle>
            <DialogDescription>
              {t('proposals.descriptions.create')}
            </DialogDescription>
          </DialogHeader>
          <Separator />
          <CreateProposalForm
            channelId={channel.id}
            createProposal={createProposal}
            onSuccess={() => setIsCreateOpen(false)}
            onNavigate={() => dialogRef.current?.scrollTo({ top: 0 })}
          />
        </DialogContent>
      </Dialog>
    </>
  );
};
