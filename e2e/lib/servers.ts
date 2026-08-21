import { expect, type APIRequestContext } from '@playwright/test';
import { authorizationHeaders, type AuthenticatedUser } from './auth';
import { ensureInstanceAdminRole } from './instance-roles';

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
