import { expect, type APIRequestContext } from '@playwright/test';
import { authorizationHeaders, type AuthenticatedUser } from './auth';
import { ensureServerAdminRole } from './servers';

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
  await ensureServerAdminRole(request, user, serverId);

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

export async function createForumPosts(
  request: APIRequestContext,
  user: AuthenticatedUser,
  serverId: string,
  channelId: string,
  titles: string[],
) {
  for (const title of titles) {
    const response = await request.post(
      `/api/servers/${serverId}/channels/${channelId}/forum/posts`,
      {
        headers: authorizationHeaders(user),
        data: {
          title,
          body: `Opening message for ${title}`,
        },
      },
    );
    await expect(response).toBeOK();
  }
}
