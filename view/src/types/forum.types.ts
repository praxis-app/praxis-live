import { type MessageRes } from './message.types';
import { type CreatePollReq, type PollRes } from './poll.types';
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
  proposal: PollRes | null;
}

export interface CreateForumPostReq {
  title: string;
  body: string;
  proposal?: CreatePollReq;
}

export interface UpdateForumPostReq {
  title?: string;
  body?: string;
}

export interface CreateForumReplyReq {
  body: string;
  imageCount: number;
  parentMessageId?: string;
}
