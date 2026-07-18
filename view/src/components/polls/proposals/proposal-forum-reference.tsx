import { Card } from '@/components/ui/card';
import { UserAvatar } from '@/components/users/user-avatar';
import { UserProfileDrawer } from '@/components/users/user-profile-drawer';
import { useServerData } from '@/hooks/use-server-data';
import { truncate } from '@/lib/text.utils';
import { timeAgo } from '@/lib/time.utils';
import { type ProposalForumReferenceRes } from '@/types/forum.types';
import { type CurrentUser } from '@/types/user.types';
import { useTranslation } from 'react-i18next';
import { MdArrowForward, MdForum } from 'react-icons/md';
import { Link } from 'react-router-dom';

interface Props {
  reference: ProposalForumReferenceRes;
  me?: CurrentUser;
}

export const ProposalForumReference = ({ reference, me }: Props) => {
  const { t } = useTranslation();
  const { serverPath } = useServerData();
  const user = reference.user;
  const name = user?.displayName || user?.name;
  const truncatedName = name ? truncate(name, 18) : undefined;

  return (
    <article className="flex max-w-full min-w-0 gap-4 pt-1">
      {user && name && truncatedName ? (
        <UserProfileDrawer
          name={truncatedName}
          userId={user.id}
          me={me}
          trigger={
            <button className="shrink-0 cursor-pointer self-start">
              <UserAvatar
                name={name}
                userId={user.id}
                imageId={user.profilePicture?.id}
                className="mt-0.5"
              />
            </button>
          }
        />
      ) : (
        <div className="bg-muted text-muted-foreground mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-full">
          <MdForum className="size-5" />
        </div>
      )}
      <div className="max-w-full min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-1.5 pb-1">
          {user && truncatedName && (
            <UserProfileDrawer
              name={truncatedName}
              userId={user.id}
              me={me}
              trigger={
                <button className="cursor-pointer font-medium">
                  {truncatedName}
                </button>
              }
            />
          )}
          <div className="text-muted-foreground text-sm">
            {timeAgo(reference.createdAt)}
          </div>
        </div>
        <Card className="bg-muted/30 max-w-full flex-row items-center gap-3 rounded-lg px-3.5 py-3 shadow-none">
          <div className="bg-primary/10 text-primary flex size-9 shrink-0 items-center justify-center rounded-full">
            <MdForum className="size-5" />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-sm font-medium">
              {t('forums.prompts.movedProposal')}
            </p>
            <Link
              className="text-primary mt-0.5 flex w-fit items-center gap-1 text-sm font-medium underline-offset-4 hover:underline"
              to={`${serverPath}/c/${reference.destinationChannelId}/posts/${reference.forumPostId}`}
            >
              {t('forums.actions.viewPost')}
              <MdArrowForward className="size-4" />
            </Link>
          </div>
        </Card>
      </div>
    </article>
  );
};
