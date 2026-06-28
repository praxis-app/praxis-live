import {
  expect,
  type APIRequestContext,
  type BrowserContext,
} from '@playwright/test';
import { ACCESS_TOKEN_KEY, createTestUser, type TestUser } from './data';
import { createInvite } from './invites';
import { AuthenticatedUser, authorizationHeaders } from './auth';

type ServerRole = {
  id: string;
  name: string;
  color: string;
  permissions: { subject: string; action: string[] }[];
  members: { id: string; name: string; displayName?: string | null }[];
};

type ServerRoleResponse = {
  serverRole: ServerRole;
};

type ServerRolesResponse = {
  serverRoles: ServerRole[];
};

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

export async function getAdminRole(
  request: APIRequestContext,
  user: AuthenticatedUser,
  serverId: string,
) {
  const response = await request.get(`/api/servers/${serverId}/roles`, {
    headers: authorizationHeaders(user),
  });

  await expect(response).toBeOK();
  const body = (await response.json()) as ServerRolesResponse;
  const adminRole = body.serverRoles.find((role) => role.name === 'admin');
  expect(adminRole).toBeTruthy();
  return adminRole!;
}

export async function getServerRole(
  request: APIRequestContext,
  user: AuthenticatedUser,
  serverId: string,
  serverRoleId: string,
) {
  const response = await request.get(
    `/api/servers/${serverId}/roles/${serverRoleId}`,
    {
      headers: authorizationHeaders(user),
    },
  );

  await expect(response).toBeOK();
  return ((await response.json()) as ServerRoleResponse).serverRole;
}
