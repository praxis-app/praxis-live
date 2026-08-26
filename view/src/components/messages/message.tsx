import { AttachedImageList } from '@/components/images/attached-image-list';
import { FormattedText } from '@/components/shared/formatted-text';
import { MessageReplyButton } from '@/components/messages/message-reply-button';
import { MessageThreadSummary } from '@/components/messages/message-thread-summary';
import { UserAvatar } from '@/components/users/user-avatar';
import { UserProfileDrawer } from '@/components/users/user-profile-drawer';
import { timeAgo } from '@/lib/time.utils';
import { type MessageRes } from '@/types/message.types';
import { useTranslation } from 'react-i18next';
import { truncate } from '../../lib/text.utils';
import { type CurrentUser } from '../../types/user.types';

interface Props {
  message: MessageRes;
  me?: CurrentUser;
  serverId?: string;
  channelId?: string;
  onOpenThread?: (rootMessageId: string) => void;
}

export const Message = ({
  message: { id, body, images, user, createdAt, replyCount, replyUsers },
  serverId,
  channelId,
  me,
  onOpenThread,
}: Props) => {
  const { t } = useTranslation();

  if (!user) {
    return null;
  }

  const formattedDate = timeAgo(createdAt);
  const showImages = !!images?.length;

  const name = user.displayName || user.name;
  const truncatedUsername = truncate(name, 18);

  return (
    <div
      data-message-id={id}
      className="group/message relative flex max-w-full min-w-0 gap-4 pt-1"
    >
      {onOpenThread && <MessageReplyButton onReply={() => onOpenThread(id)} />}

      <UserProfileDrawer
        name={truncatedUsername}
        userId={user.id}
        me={me}
        trigger={
          <button className="shrink-0 cursor-pointer self-start">
            <UserAvatar
              name={name}
              userId={user.id}
              className="mt-0.5"
              imageId={user.profilePicture?.id}
            />
          </button>
        }
      />

      <div className="max-w-full min-w-0 flex-1">
        <div className="mb-[-0.1rem] flex min-w-0 items-center gap-1.5">
          <UserProfileDrawer
            name={truncatedUsername}
            userId={user.id}
            me={me}
            trigger={
              <button className="cursor-pointer font-medium">
                {truncatedUsername}
              </button>
            }
          />
          <div className="text-muted-foreground text-sm font-light">
            {formattedDate}
          </div>
        </div>

        {/* TODO: Truncate message body if it exceeds a certain length */}
        {body && <FormattedText text={body} />}

        {/* TODO: Enable navigation between images in modal */}
        {showImages && (
          <AttachedImageList
            images={images}
            serverId={serverId}
            channelId={channelId}
            messageId={id}
            imageClassName="rounded-lg"
            className="w-full max-w-[min(350px,100%)] pt-1.5"
          />
        )}

        {!body && !showImages && (
          <div className="text-muted-foreground text-sm">
            {t('prompts.noContent')}
          </div>
        )}

        {onOpenThread && replyCount > 0 && (
          <MessageThreadSummary
            replyCount={replyCount}
            replyUsers={replyUsers || []}
            onOpen={() => onOpenThread(id)}
          />
        )}
      </div>
    </div>
  );
};
