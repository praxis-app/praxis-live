export interface PublicUser {
  id: number;
  email: string;
  name: string;
}

export interface SessionResponse {
  user: PublicUser | null;
  access_token?: string | null;
}

export interface Channel {
  id: string;
  name: string;
  description: string | null;
  server: {
    id: string;
    slug: string;
  };
}

export interface MessageImage {
  id: string;
  isPlaceholder?: boolean;
  createdAt: string;
}

export interface Message {
  id: string;
  body: string | null;
  images?: MessageImage[];
  user: {
    id: string;
    name: string;
    profile_picture: unknown;
  } | null;
  userId: string | null;
  botId: string | null;
  bot: unknown;
  commandStatus?: string | null;
  createdAt: string;
}

export interface FeedMessage extends Message {
  type: "message";
}
