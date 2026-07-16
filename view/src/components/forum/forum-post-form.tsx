import { api } from '@/client/api-client';
import { Button } from '@/components/ui/button';
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { useServerData } from '@/hooks/use-server-data';
import { handleError } from '@/lib/error.utils';
import { type ChannelRes } from '@/types/channel.types';
import { type PollRes } from '@/types/poll.types';
import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import * as zod from 'zod';

const schema = zod.object({
  title: zod.string().trim().min(1).max(100),
  body: zod.string().trim().min(1).max(6000),
  pollId: zod.string(),
});

interface Props {
  channel: ChannelRes;
  onSuccess: () => void;
}

export const ForumPostForm = ({ channel, onSuccess }: Props) => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { serverId, serverPath } = useServerData();
  const form = useForm<zod.infer<typeof schema>>({
    resolver: zodResolver(schema),
    defaultValues: { title: '', body: '', pollId: 'none' },
  });

  const { data: feedData } = useQuery({
    queryKey: [
      'servers',
      serverId,
      'channels',
      channel.id,
      'forum-proposals',
    ],
    queryFn: () => {
      if (!serverId) throw new Error('Server ID is required');
      return api.getChannelFeed(serverId, channel.id, 0, 100);
    },
    enabled: !!serverId,
  });
  const proposals =
    feedData?.feed.filter(
      (item): item is PollRes & { type: 'poll' } =>
        item.type === 'poll' && item.pollType === 'proposal',
    ) ?? [];

  const { mutate: createPost, isPending } = useMutation({
    mutationFn: (values: zod.infer<typeof schema>) => {
      if (!serverId) throw new Error('Server ID is required');
      return api.createForumPost(serverId, channel.id, {
        title: values.title,
        body: values.body,
        pollId: values.pollId === 'none' ? undefined : values.pollId,
      });
    },
    onSuccess: ({ post }) => {
      void queryClient.invalidateQueries({
        queryKey: ['servers', serverId, 'channels', channel.id, 'forum'],
      });
      onSuccess();
      navigate(`${serverPath}/c/${channel.id}/posts/${post.id}`);
    },
    onError: handleError,
  });

  return (
    <Form {...form}>
      <form
        className="space-y-4"
        onSubmit={form.handleSubmit((values) => createPost(values))}
      >
        <FormField
          control={form.control}
          name="title"
          render={({ field }) => (
            <FormItem>
              <FormLabel>{t('forums.form.title')}</FormLabel>
              <FormControl>
                <Input {...field} autoFocus />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />
        <FormField
          control={form.control}
          name="body"
          render={({ field }) => (
            <FormItem>
              <FormLabel>{t('forums.form.body')}</FormLabel>
              <FormControl>
                <Textarea {...field} rows={6} />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />
        <FormField
          control={form.control}
          name="pollId"
          render={({ field }) => (
            <FormItem>
              <FormLabel>{t('forums.form.proposal')}</FormLabel>
              <Select onValueChange={field.onChange} value={field.value}>
                <FormControl>
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                </FormControl>
                <SelectContent>
                  <SelectItem value="none">
                    {t('forums.labels.noProposal')}
                  </SelectItem>
                  {proposals.map((proposal) => (
                    <SelectItem key={proposal.id} value={proposal.id}>
                      {proposal.body || t('forums.labels.untitledProposal')}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <FormMessage />
            </FormItem>
          )}
        />
        <Button type="submit" disabled={isPending}>
          {isPending
            ? t('forums.actions.creatingPost')
            : t('forums.actions.createPost')}
        </Button>
      </form>
    </Form>
  );
};
