import { expect, type APIRequestContext } from '@playwright/test';
import { authorizationHeaders, type AuthenticatedUser } from './auth';

type ForumChannel = {
  id: string;
  name: string;
  channelType: 'forum';
};

export async function createForumChannel(
  request: APIRequestContext,
  user: AuthenticatedUser,
  serverId: string,
  name: string,
) {
  const response = await request.post(`/api/servers/${serverId}/channels`, {
    headers: authorizationHeaders(user),
    data: {
      name,
      description: `E2E forum channel ${name}`,
      channelType: 'forum',
    },
  });

  await expect(response).toBeOK();
  const channel = ((await response.json()) as { channel: ForumChannel })
    .channel;
  expect(channel.name).toBe(name);
  expect(channel.channelType).toBe('forum');
  return channel;
}
