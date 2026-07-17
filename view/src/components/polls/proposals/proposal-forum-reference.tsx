import { Card } from '@/components/ui/card';
import { useServerData } from '@/hooks/use-server-data';
import { timeAgo } from '@/lib/time.utils';
import { type ProposalForumReferenceRes } from '@/types/forum.types';
import { useTranslation } from 'react-i18next';
import { MdArrowForward, MdForum } from 'react-icons/md';
import { Link } from 'react-router-dom';

interface Props {
  reference: ProposalForumReferenceRes;
}

export const ProposalForumReference = ({ reference }: Props) => {
  const { t } = useTranslation();
  const { serverPath } = useServerData();

  return (
    <article className="flex max-w-full min-w-0 gap-4 pt-1">
      <div className="bg-muted mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-full">
        <MdForum className="text-muted-foreground size-5" />
      </div>
      <div className="max-w-full min-w-0 flex-1">
        <div className="text-muted-foreground pb-1 text-sm">
          {timeAgo(reference.createdAt)}
        </div>
        <Card className="max-w-full gap-2 rounded-md px-4 py-3 shadow-none">
          <p className="text-sm">{t('forums.prompts.movedProposal')}</p>
          <Link
            className="text-primary flex w-fit items-center gap-1 text-sm font-medium underline-offset-4 hover:underline"
            to={`${serverPath}/c/${reference.destinationChannelId}/posts/${reference.forumPostId}`}
          >
            {t('forums.actions.viewPost')}
            <MdArrowForward />
          </Link>
        </Card>
      </div>
    </article>
  );
};
