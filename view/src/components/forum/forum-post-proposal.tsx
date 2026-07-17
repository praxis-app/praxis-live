import { api } from '@/client/api-client';
import { CreateProposalForm } from '@/components/polls/proposals/create-proposal-form/create-proposal-form';
import { InlineProposal } from '@/components/polls/proposals/inline-proposal/inline-proposal';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Separator } from '@/components/ui/separator';
import { useAuthData } from '@/hooks/use-auth-data';
import { useServerData } from '@/hooks/use-server-data';
import { type ChannelRes } from '@/types/channel.types';
import { type ForumPostRes } from '@/types/forum.types';
import { type CreatePollReq } from '@/types/poll.types';
import { type QueryKey, useQueryClient } from '@tanstack/react-query';
import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

interface Props {
  channel: ChannelRes;
  post: ForumPostRes;
  feedQueryKey: QueryKey;
}

export const ForumPostProposal = ({
  channel,
  post,
  feedQueryKey,
}: Props) => {
  const { t } = useTranslation();
  const { me } = useAuthData();
  const { serverId } = useServerData();
  const queryClient = useQueryClient();
  const dialogRef = useRef<HTMLDivElement>(null);
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const isAuthor = me?.id === post.user.id;

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
    void queryClient.invalidateQueries({
      queryKey: ['servers', serverId, 'channels', channel.id, 'forum'],
    });
    return { poll: updatedPost.proposal };
  };

  return (
    <section className="space-y-3 rounded-lg border p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="font-medium">{t('forums.labels.proposal')}</h2>
        {isAuthor && !post.proposal && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => setIsCreateOpen(true)}
          >
            {t('forums.actions.createProposalFromDiscussion')}
          </Button>
        )}
      </div>

      {post.proposal ? (
        <InlineProposal
          poll={post.proposal}
          channel={channel}
          feedQueryKey={feedQueryKey}
          me={me}
        />
      ) : (
        <p className="text-muted-foreground text-sm">
          {t('forums.prompts.noProposal')}
        </p>
      )}

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
    </section>
  );
};
