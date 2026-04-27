import {
  expect,
  type APIRequestContext,
  type BrowserContext,
} from '@playwright/test';
import { ACCESS_TOKEN_KEY, createTestUser, type TestUser } from './data';
import { createInvite } from './invites';
import { AuthenticatedUser, authorizationHeaders } from './auth';

export async function getDefaultServer(
  request: APIRequestContext,
  user: AuthenticatedUser,
) {
  const response = await request.get('/api/servers/default', {
    headers: authorizationHeaders(user),
  });

  await expect(response).toBeOK();
  return (await response.json()).server;
}

export async function enableAnonymousUsers(
  request: APIRequestContext,
  user: AuthenticatedUser,
  serverId: string,
) {
  const response = await request.put(`/api/servers/${serverId}/configs`, {
    headers: authorizationHeaders(user),
    data: { anonymousUsersEnabled: true },
  });

  await expect(response).toBeOK();
}
