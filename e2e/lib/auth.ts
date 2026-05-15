import {
  expect,
  type APIRequestContext,
  type BrowserContext,
} from '@playwright/test';
import { ACCESS_TOKEN_KEY, createTestUser, type TestUser } from './data';
import { createInvite } from './invites';
import { enableAnonymousUsers, getDefaultServer } from './servers';

export type AuthenticatedUser = {
  accessToken: string;
  userId: string;
  user: TestUser;
};

export type AnonymousAuthSetup = {
  admin: AuthenticatedUser;
  server: {
    id: string;
    generalChannelId: string;
  };
  inviteToken: string;
};

type SignupResponse = {
  access_token?: string | null;
  user?: {
    id: string;
    email: string;
    name: string;
  } | null;
};

type AuthResponse = {
  access_token?: string | null;
};

type ServerResponse = {
  server: {
    id: string;
    generalChannelId: string;
  };
};

export async function signUpViaApi(
  request: APIRequestContext,
  user: TestUser = createTestUser(),
  inviteToken?: string,
): Promise<AuthenticatedUser> {
  const response = await request.post('/api/auth/signup', {
    data: {
      name: user.name,
      email: user.email,
      password: user.password,
      inviteToken,
    },
  });

  await expect(response).toBeOK();
  const body = (await response.json()) as SignupResponse;
  expect(body.access_token).toBeTruthy();
  expect(body.user?.id).toBeTruthy();
  expect(body.user?.email).toBe(user.email);

  return {
    accessToken: body.access_token ?? '',
    userId: body.user?.id ?? '',
    user,
  };
}

export async function seedAuthenticatedSession(
  context: BrowserContext,
  accessToken: string,
) {
  await context.addInitScript(
    ([key, token]) => {
      window.localStorage.setItem(key, token);
    },
    [ACCESS_TOKEN_KEY, accessToken],
  );
}

export async function createAuthenticatedUser(
  request: APIRequestContext,
  context: BrowserContext,
  user: TestUser = createTestUser(),
) {
  const authenticatedUser = await signUpViaApi(request, user);
  await seedAuthenticatedSession(context, authenticatedUser.accessToken);

  return authenticatedUser;
}

export async function setupAnonymousInvite(
  request: APIRequestContext,
  context: BrowserContext,
  adminLabel: string,
): Promise<AnonymousAuthSetup> {
  const admin = await createAuthenticatedUser(
    request,
    context,
    createTestUser(adminLabel),
  );
  const server = await getDefaultServer(request, admin);
  await enableAnonymousUsers(request, admin, server.id);
  const inviteToken = await createInvite(request, admin, server.id);

  await context.clearCookies();
  await context.addInitScript(
    ([accessTokenKey, inviteTokenValue]) => {
      window.localStorage.removeItem(accessTokenKey);
      window.localStorage.setItem('invite-token', inviteTokenValue);
    },
    [ACCESS_TOKEN_KEY, inviteToken],
  );

  return { admin, server, inviteToken };
}

export async function setupAnonymousSession(
  request: APIRequestContext,
  context: BrowserContext,
  adminLabel: string,
) {
  const anonymous = await setupAnonymousInvite(request, context, adminLabel);
  const response = await request.post('/api/auth/anon', {
    data: { inviteToken: anonymous.inviteToken },
  });

  await expect(response).toBeOK();
  const body = (await response.json()) as AuthResponse;
  expect(body.access_token).toBeTruthy();

  await context.addInitScript(
    ([accessTokenKey, accessToken]) => {
      window.localStorage.removeItem('invite-token');
      window.localStorage.setItem(accessTokenKey, accessToken);
    },
    [ACCESS_TOKEN_KEY, body.access_token ?? ''],
  );

  return anonymous;
}

export function authorizationHeaders(user: AuthenticatedUser) {
  return {
    Authorization: `Bearer ${user.accessToken}`,
  };
}
