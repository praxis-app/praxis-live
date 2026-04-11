import {
  expect,
  type APIRequestContext,
  type BrowserContext,
} from "@playwright/test";
import { ACCESS_TOKEN_KEY, createTestUser, type TestUser } from "./test-data";

export type AuthenticatedUser = {
  accessToken: string;
  user: TestUser;
};

type SignupResponse = {
  access_token?: string | null;
  user?: {
    email: string;
    name: string;
  } | null;
};

export async function signUpViaApi(
  request: APIRequestContext,
  user: TestUser = createTestUser()
): Promise<AuthenticatedUser> {
  const response = await request.post("/api/auth/signup", {
    data: {
      name: user.name,
      email: user.email,
      password: user.password,
    },
  });

  await expect(response).toBeOK();
  const body = (await response.json()) as SignupResponse;
  expect(body.access_token).toBeTruthy();
  expect(body.user?.email).toBe(user.email);

  return {
    accessToken: body.access_token ?? "",
    user,
  };
}

export async function seedAuthenticatedSession(
  context: BrowserContext,
  accessToken: string
) {
  await context.addInitScript(
    ([key, token]) => {
      window.localStorage.setItem(key, token);
    },
    [ACCESS_TOKEN_KEY, accessToken]
  );
}

export async function createAuthenticatedUser(
  request: APIRequestContext,
  context: BrowserContext,
  user: TestUser = createTestUser()
) {
  const authenticatedUser = await signUpViaApi(request, user);
  await seedAuthenticatedSession(context, authenticatedUser.accessToken);

  return authenticatedUser;
}
