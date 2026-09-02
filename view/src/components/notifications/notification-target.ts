import { type NotificationRes } from '@/types/notification.types';

export const getNotificationTargetPath = (
  notification: NotificationRes,
  serverSlug: string,
) => {
  const { target } = notification;
  if (!target.available) return null;

  if (target.kind === 'serverRole') {
    return `/s/${serverSlug}`;
  }
  if (!target.channelId) return null;

  const channelPath = `/s/${serverSlug}/c/${target.channelId}`;
  if (target.forumPostId) {
    const reply =
      target.threadRootId && target.messageId
        ? `?reply=${target.messageId}`
        : '';
    return `${channelPath}/posts/${target.forumPostId}${reply}`;
  }
  if (target.threadRootId) {
    const kind = target.threadRootKind === 'poll' ? '&threadKind=poll' : '';
    return `${channelPath}?thread=${target.threadRootId}${kind}`;
  }
  if (target.pollId) {
    return `${channelPath}?thread=${target.pollId}&threadKind=poll`;
  }
  if (target.messageId) {
    return `${channelPath}?message=${target.messageId}`;
  }
  return channelPath;
};
