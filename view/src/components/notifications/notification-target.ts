import { type NotificationRes } from '@/types/notification.types';

export interface NotificationTargetRoute {
  path: string;

  /** Focuses a decision in the channel feed, matching the decisions panel. */
  state?: { decisionId: string };
}

export const getNotificationTargetRoute = (
  notification: NotificationRes,
  serverSlug: string,
): NotificationTargetRoute | null => {
  const { target } = notification;
  if (!target.available) return null;

  if (target.kind === 'serverRole') {
    return { path: `/s/${serverSlug}` };
  }
  if (!target.channelId) return null;

  const channelPath = `/s/${serverSlug}/c/${target.channelId}`;
  if (target.forumPostId) {
    const reply =
      target.threadRootId && target.messageId
        ? `?reply=${target.messageId}`
        : '';
    return { path: `${channelPath}/posts/${target.forumPostId}${reply}` };
  }
  // A reply opens its thread panel, and focuses the thread root in the feed
  // behind it so the conversation is still placed in the channel.
  if (target.threadRootId) {
    const params = new URLSearchParams({ thread: target.threadRootId });
    if (target.messageId) {
      params.set('reply', target.messageId);
    }
    if (target.threadRootKind === 'poll') {
      params.set('threadKind', 'poll');
      return {
        path: `${channelPath}?${params}`,
        state: { decisionId: target.threadRootId },
      };
    }
    params.set('message', target.threadRootId);
    return { path: `${channelPath}?${params}` };
  }
  if (target.pollId) {
    return { path: channelPath, state: { decisionId: target.pollId } };
  }
  if (target.messageId) {
    return { path: `${channelPath}?message=${target.messageId}` };
  }
  return { path: channelPath };
};
