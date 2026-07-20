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
  createdAt: string;
}

export interface MessagesQuery {
  pages: { messages: MessageRes[] }[];
  pageParams: number[];
}
