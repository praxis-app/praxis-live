import { type ImageRes } from './image.types';

export type NotificationKind =
  | 'new_message'
  | 'message_reply'
  | 'forum_reply'
  | 'proposal_vote'
  | 'proposal_ratified'
  | 'proposal_closed'
  | 'server_role_granted';

export type NotificationVoteType = 'agree' | 'disagree' | 'abstain' | 'block';

export interface NotificationActor {
  id: string;
  name: string;
  displayName?: string;
  profilePicture: ImageRes | null;
}

export interface NotificationTarget {
  kind: 'message' | 'poll' | 'serverRole' | 'unavailable';
  available: boolean;
  channelId?: string;
  channelName?: string;
  messageId?: string;
  threadRootId?: string;
  threadRootKind?: 'message' | 'poll';
  forumPostId?: string;
  pollId?: string;
  serverRoleId?: string;
  serverRoleName?: string;
}

export interface NotificationRes {
  id: string;
  kind: NotificationKind;
  serverId: string;
  channelId: string | null;
  actor: NotificationActor | null;
  voteType: NotificationVoteType | null;
  unreadCount: number | null;
  readAt: string | null;
  createdAt: string;
  target: NotificationTarget;
}

export interface NotificationsPageRes {
  notifications: NotificationRes[];
  nextCursor: string | null;
  hasMore: boolean;
}

export interface NotificationPayload {
  notification: NotificationRes;
}

export interface UnreadNotificationCountRes {
  unreadCount: number;
}
