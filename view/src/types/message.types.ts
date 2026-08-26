import { type CommandStatus } from '@/types/command.types';
import { type ImageRes } from './image.types';
import { type UserRes } from './user.types';

export interface BotRes {
  id: string;
  name: string;
  displayName: string | null;
}

export interface MessageRes {
  id: string;
  body: string | null;
  images?: ImageRes[];
  user: UserRes | null;
  userId: string | null;
  botId: string | null;
  bot: BotRes | null;
  commandStatus?: CommandStatus | null;
  threadRootId?: string;
  parentMessageId?: string;
  replyCount: number;
  latestReplyAt: string | null;
  createdAt: string;
}

export interface CreateReplyReq {
  body?: string;
  parentMessageId?: string;
}

export interface ThreadPageRes {
  root: MessageRes;
  replies: MessageRes[];
  startCursor: string | null;
  nextCursor: string | null;
  hasMore: boolean;
}

export interface ThreadQuery {
  pages: ThreadPageRes[];
  pageParams: (string | null)[];
}
