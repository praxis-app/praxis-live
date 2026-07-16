import { type MessageRes } from './message.types';
import { type UserRes } from './user.types';

export type ForumPostSort = 'recent' | 'newest';
export type ForumPostStatus = 'open' | 'closed';

export interface ForumPostSummaryRes {
  id: string;
  title: string;
  rootMessageId: string;
  pollId?: string;
  status: ForumPostStatus;
  user: UserRes;
  replyCount: number;
  latestActivityAt: string;
  createdAt: string;
  updatedAt: string;
}

export interface ForumPostRes extends ForumPostSummaryRes {
  body: string;
  replies: MessageRes[];
}

export interface CreateForumPostReq {
  title: string;
  body: string;
  pollId?: string;
}

export interface UpdateForumPostReq {
  title?: string;
  body?: string;
  pollId?: string | null;
}

export interface CreateForumReplyReq {
  body: string;
  imageCount: number;
  parentMessageId?: string;
}
