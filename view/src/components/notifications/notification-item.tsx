import { UserAvatar } from '@/components/users/user-avatar';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { cn } from '@/lib/shared.utils';
import { timeAgo } from '@/lib/time.utils';
import { type NotificationRes } from '@/types/notification.types';
import { useTranslation } from 'react-i18next';
import {
  LuAtSign,
  LuBadgeCheck,
  LuCheck,
  LuCircleDot,
  LuMessageCircle,
  LuEllipsis,
  LuReply,
  LuTrash2,
  LuVote,
} from 'react-icons/lu';

interface Props {
  notification: NotificationRes;
  onSelect: (notification: NotificationRes) => void;
  onMarkRead: (notification: NotificationRes) => void;
  onMarkUnread: (notification: NotificationRes) => void;
  onDelete: (notification: NotificationRes) => void;
}

const getIcon = (kind: NotificationRes['kind'], className: string) => {
  switch (kind) {
    case 'new_message':
      return <LuAtSign className={className} />;
    case 'message_reply':
    case 'forum_reply':
      return <LuReply className={className} />;
    case 'proposal_vote':
      return <LuVote className={className} />;
    case 'proposal_ratified':
    case 'proposal_closed':
      return <LuBadgeCheck className={className} />;
    case 'server_role_granted':
      return <LuCircleDot className={className} />;
    default:
      return <LuMessageCircle className={className} />;
  }
};

export const NotificationItem = ({
  notification,
  onSelect,
  onMarkRead,
  onMarkUnread,
  onDelete,
}: Props) => {
  const { t } = useTranslation();
  const actorName =
    notification.actor?.displayName || notification.actor?.name ||
    t('notifications.labels.system');
  const isUnread = !notification.readAt;
  const isAvailable = notification.target.available;
  const count = notification.unreadCount || 1;
  const description = t(`notifications.items.${notification.kind}`, {
    actor: actorName,
    channel: notification.target.channelName,
    role: notification.target.serverRoleName,
    vote: notification.voteType
      ? t(`notifications.votes.${notification.voteType}`)
      : undefined,
    count,
  });

  return (
    <div
      data-testid="notification-item"
      data-unread={isUnread || undefined}
      className={cn(
        'group relative flex items-start gap-2 border-b px-3 py-3',
        isUnread && 'bg-primary/5 before:bg-primary before:absolute before:inset-y-2 before:left-0 before:w-0.5 before:rounded-full',
      )}
    >
      <div className="relative mt-0.5 shrink-0">
        {notification.actor ? (
          <UserAvatar
            className="size-9"
            fallbackClassName="text-sm"
            name={actorName}
            userId={notification.actor.id}
            imageId={notification.actor.profilePicture?.id}
          />
        ) : (
          <div className="bg-muted flex size-9 items-center justify-center rounded-full">
            {getIcon(notification.kind, 'size-4.5')}
          </div>
        )}
        {notification.actor && (
          <span className="bg-background absolute -right-1 -bottom-1 flex size-5 items-center justify-center rounded-full border shadow-sm">
            {getIcon(notification.kind, 'size-3')}
          </span>
        )}
      </div>

      <button
        type="button"
        disabled={!isAvailable}
        onClick={() => onSelect(notification)}
        className="min-w-0 flex-1 text-left disabled:cursor-default"
      >
        <p className={cn('text-sm leading-5', isUnread && 'font-medium')}>
          {description}
        </p>
        <p className="text-muted-foreground mt-0.5 text-xs">
          {isAvailable
            ? timeAgo(notification.createdAt)
            : t('notifications.labels.unavailable')}
        </p>
      </button>

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="-mt-1 size-8 shrink-0"
            aria-label={t('notifications.actions.openMenu')}
          >
            <LuEllipsis />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem
            onSelect={() =>
              isUnread
                ? onMarkRead(notification)
                : onMarkUnread(notification)
            }
          >
            <LuCheck />
            {isUnread
              ? t('notifications.actions.markRead')
              : t('notifications.actions.markUnread')}
          </DropdownMenuItem>
          <DropdownMenuItem
            variant="destructive"
            onSelect={() => onDelete(notification)}
          >
            <LuTrash2 />
            {t('notifications.actions.delete')}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
};
