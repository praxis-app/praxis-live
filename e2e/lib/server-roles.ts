import { expect, type APIRequestContext } from '@playwright/test';
import { authorizationHeaders, type AuthenticatedUser } from './auth';
import { grantViaNewRole, type PermissionRule } from './permissions';

type ServerRole = {
  id: string;
  name: string;
  color: string;
  permissions: PermissionRule[];
  members: { id: string; name: string; displayName?: string | null }[];
};

type ServerRoleResponse = {
  serverRole: ServerRole;
};

type ServerRolesResponse = {
  serverRoles: ServerRole[];
};

export async function grantServerPermissions(
  request: APIRequestContext,
  granter: AuthenticatedUser,
  user: AuthenticatedUser,
  serverId: string,
  permissions: PermissionRule[],
  label: string,
) {
  if (granter.userId === user.userId) {
    return;
  }

  await grantViaNewRole(
    request,
    granter,
    `/api/servers/${serverId}/roles`,
    'serverRole',
    user,
    permissions,
    label,
  );
}

export async function getAdminServerRole(
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

export async function createServerRole(
  request: APIRequestContext,
  granter: AuthenticatedUser,
  serverId: string,
  name: string,
) {
  const response = await request.post(`/api/servers/${serverId}/roles`, {
    headers: authorizationHeaders(granter),
    data: { name, color: '#2196f3' },
  });

  await expect(response).toBeOK();
  return ((await response.json()) as ServerRoleResponse).serverRole;
}

export async function addServerRoleMembers(
  request: APIRequestContext,
  granter: AuthenticatedUser,
  serverId: string,
  serverRoleId: string,
  userIds: string[],
) {
  const response = await request.post(
    `/api/servers/${serverId}/roles/${serverRoleId}/members`,
    {
      headers: authorizationHeaders(granter),
      data: { userIds },
    },
  );

  await expect(response).toBeOK();
}
