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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useAuthData } from '@/hooks/use-auth-data';
import { useServerData } from '@/hooks/use-server-data';
import { handleError } from '@/lib/error.utils';
import { type ChannelRes, type FeedQuery } from '@/types/channel.types';
import { type ForumPostRes } from '@/types/forum.types';
import { type PollRes } from '@/types/poll.types';
import {
  type QueryKey,
  useMutation,
  useQueryClient,
} from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

interface Props {
  channel: ChannelRes;
  post: ForumPostRes;
  proposals: PollRes[];
  feedQueryKey: QueryKey;
}

export const ForumPostProposal = ({
  channel,
  post,
  proposals,
  feedQueryKey,
}: Props) => {
  const { t } = useTranslation();
  const { me } = useAuthData();
  const { serverId } = useServerData();
  const queryClient = useQueryClient();
  const dialogRef = useRef<HTMLDivElement>(null);
  const [selectedPollId, setSelectedPollId] = useState(post.pollId ?? 'none');
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const isAuthor = me?.id === post.user.id;
  const linkedProposal = proposals.find((poll) => poll.id === post.pollId);

  useEffect(() => {
    setSelectedPollId(post.pollId ?? 'none');
  }, [post.pollId]);

  const { mutate: associateProposal, isPending } = useMutation({
    mutationFn: (pollId: string | null) => {
      if (!serverId) throw new Error('Server ID is required');
      return api.updateForumPost(serverId, channel.id, post.id, { pollId });
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [
          'servers',
          serverId,
          'channels',
          channel.id,
          'forum',
        ],
      });
    },
    onError: handleError,
  });

  const handleCreatedProposal = (poll: PollRes) => {
    queryClient.setQueryData<FeedQuery>(feedQueryKey, (old) => {
      if (old) return old;
      return {
        pages: [{ feed: [{ ...poll, type: 'poll' as const }] }],
        pageParams: [0],
      };
    });
    setIsCreateOpen(false);
    setSelectedPollId(poll.id);
    associateProposal(poll.id);
  };

  return (
    <section className="space-y-3 rounded-lg border p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="font-medium">{t('forums.labels.proposal')}</h2>
        {isAuthor && (
          <Button variant="outline" size="sm" onClick={() => setIsCreateOpen(true)}>
            {t('forums.actions.createProposal')}
          </Button>
        )}
      </div>

      {isAuthor && (
        <div className="flex flex-col gap-2 sm:flex-row">
          <Select value={selectedPollId} onValueChange={setSelectedPollId}>
            <SelectTrigger className="min-w-0 flex-1">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">{t('forums.labels.noProposal')}</SelectItem>
              {proposals.map((proposal) => (
                <SelectItem key={proposal.id} value={proposal.id}>
                  {proposal.body || t('forums.labels.untitledProposal')}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button
            disabled={isPending || selectedPollId === (post.pollId ?? 'none')}
            onClick={() =>
              associateProposal(selectedPollId === 'none' ? null : selectedPollId)
            }
          >
            {t('forums.actions.saveProposal')}
          </Button>
        </div>
      )}

      {linkedProposal ? (
        <InlineProposal
          poll={linkedProposal}
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
            <DialogTitle>{t('proposals.headers.create')}</DialogTitle>
            <DialogDescription>
              {t('proposals.descriptions.create')}
            </DialogDescription>
          </DialogHeader>
          <Separator />
          <CreateProposalForm
            channelId={channel.id}
            onSuccess={handleCreatedProposal}
            onNavigate={() => dialogRef.current?.scrollTo({ top: 0 })}
          />
        </DialogContent>
      </Dialog>
    </section>
  );
};
