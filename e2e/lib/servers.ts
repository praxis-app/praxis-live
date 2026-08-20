import { expect, type APIRequestContext } from '@playwright/test';
import {
  authorizationHeaders,
  getOrCreateInstanceAdmin,
  type AuthenticatedUser,
} from './auth';
import { ensureInstanceAdminRole } from './instance-roles';

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

type CreateServerOptions = {
  name: string;
  slug: string;
  description?: string | null;
  isDefaultServer?: boolean;
  image?: { name: string; mimeType: string; buffer: Buffer };
};

// Creating a server is an instance-scoped action, so the caller is elevated to
// the instance admin role first. Sends the request as multipart when an image
// is supplied and as JSON otherwise, matching what the app itself does.
export async function createServer(
  request: APIRequestContext,
  user: AuthenticatedUser,
  options: CreateServerOptions,
) {
  await ensureInstanceAdminRole(request, user);

  const payload = {
    name: options.name,
    slug: options.slug,
    description: options.description ?? null,
    isDefaultServer: options.isDefaultServer ?? false,
  };

  const response = await request.post('/api/servers', {
    headers: authorizationHeaders(user),
    ...(options.image
      ? {
          multipart: {
            payload: JSON.stringify(payload),
            file: options.image,
          },
        }
      : { data: payload }),
  });

  await expect(response).toBeOK();
  return (await response.json()).server;
}

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

// Finds the server role that `memberUserId` already belongs to. Unlike
// looking a role up by its literal 'admin' name, this survives role-rename
// or permission-edit proposals that other specs exercise against the
// server's original admin role (its membership is never altered by those
// flows, only its name/permissions).
async function findRoleForMember(
  request: APIRequestContext,
  caller: AuthenticatedUser,
  serverId: string,
  memberUserId: string,
) {
  const response = await request.get(`/api/servers/${serverId}/roles`, {
    headers: authorizationHeaders(caller),
  });

  if (!response.ok()) {
    return undefined;
  }

  const body = (await response.json()) as ServerRolesResponse;
  return body.serverRoles.find((role) =>
    role.members.some((member) => member.id === memberUserId),
  );
}

// Elevates `user` to the server's admin role using the instance admin as
// the granter, tolerating failure when the instance admin has no standing
// on this particular server (e.g. `user` already administers a server they
// created themselves).
export async function ensureServerAdminRole(
  request: APIRequestContext,
  user: AuthenticatedUser,
  serverId: string,
) {
  const instanceAdmin = await getOrCreateInstanceAdmin(request);
  const adminRole = await findRoleForMember(
    request,
    instanceAdmin,
    serverId,
    instanceAdmin.userId,
  );
  if (!adminRole) {
    return;
  }

  await request.post(`/api/servers/${serverId}/roles/${adminRole.id}/members`, {
    headers: authorizationHeaders(instanceAdmin),
    data: { userIds: [user.userId] },
  });
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
