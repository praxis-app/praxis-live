import {
  expect,
  test,
  type APIRequestContext,
  type Page,
} from "@playwright/test";
import {
  createAuthenticatedUser,
  type AuthenticatedUser,
} from "../lib/auth-api";
import {
  ACCESS_TOKEN_KEY,
  createTestMessage,
  createTestUser,
} from "../lib/test-data";
import { ChatPage } from "../pages/chat.page";
import { NavigationPage } from "../pages/navigation.page";

type ServerResponse = {
  server: {
    id: string;
    generalChannelId: string;
  };
};

type InviteResponse = {
  invite: {
    token: string;
  };
};

test("authenticated user can send a basic chat message", async ({
  context,
  page,
  request,
}) => {
  const authenticatedUser = await createAuthenticatedUser(
    request,
    context,
    createTestUser("chat")
  );
  const message = createTestMessage("chat", authenticatedUser.user.suffix);
  const chat = new ChatPage(page);
  const navigation = new NavigationPage(page);

  await chat.goto();

  await chat.expectChannel("general");
  await navigation.expectSignedInUser(authenticatedUser.user);
  await chat.sendMessage(message);
  await chat.expectMessage(message, authenticatedUser.user.name);
});

test("authenticated user can send a chat message with an image", async ({
  context,
  page,
  request,
}) => {
  const authenticatedUser = await createAuthenticatedUser(
    request,
    context,
    createTestUser("chat-image")
  );
  const message = createTestMessage("chat-image", authenticatedUser.user.suffix);
  const chat = new ChatPage(page);
  const navigation = new NavigationPage(page);

  await chat.goto();

  await chat.expectChannel("general");
  await navigation.expectSignedInUser(authenticatedUser.user);

  const uploadResponse = page.waitForResponse(
    (response) =>
      response.request().method() === "POST" &&
      response.url().includes("/images/") &&
      response.url().endsWith("/upload") &&
      response.status() === 201
  );

  await chat.attachImage();
  await chat.sendMessage(message);
  await uploadResponse;

  await chat.expectMessage(message, authenticatedUser.user.name);
  await chat.expectAttachedImage();
});

test("anonymous user can send messages and create only allowed chat polls", async ({
  context,
  page,
  request,
}) => {
  test.setTimeout(60_000);

  const admin = await createAuthenticatedUser(
    request,
    context,
    createTestUser("anon-admin")
  );
  const server = await getDefaultServer(request, admin);
  await enableAnonymousUsers(request, admin, server.id);
  const inviteToken = await createInvite(request, admin, server.id);

  await context.clearCookies();
  await context.addInitScript(
    ([accessTokenKey, inviteTokenValue]) => {
      window.localStorage.removeItem(accessTokenKey);
      window.localStorage.setItem("invite-token", inviteTokenValue);
    },
    [ACCESS_TOKEN_KEY, inviteToken]
  );

  const message = createTestMessage("anon-chat", admin.user.suffix);
  const pollQuestion = `Anonymous poll ${admin.user.suffix}?`;
  const proposalBody = `Anonymous test proposal ${admin.user.suffix}`;
  const chat = new ChatPage(page);

  await chat.goto();
  await chat.expectChannel("general");

  const anonSessionResponse = page.waitForResponse(
    (response) =>
      response.request().method() === "POST" &&
      response.url().endsWith("/api/auth/anon") &&
      response.status() === 200
  );
  const messageResponse = page.waitForResponse(
    (response) =>
      response.request().method() === "POST" &&
      response.url().includes(
        `/channels/${server.generalChannelId}/messages`
      ) &&
      response.status() === 200
  );

  await page.getByPlaceholder("Send a message...").fill(message);
  await page.getByPlaceholder("Send a message...").press("Enter");
  await page.getByRole("button", { name: "Send anonymously" }).click();
  await anonSessionResponse;
  await messageResponse;
  await expect(page.getByText(message)).toBeVisible();

  await openCreatePollDialog(page, "Create poll");
  const pollDialog = page.getByRole("dialog", { name: "Create a Poll" });
  await pollDialog
    .getByPlaceholder("What question do you want to ask?")
    .fill(pollQuestion);
  await pollDialog.getByPlaceholder("Answer 1").fill("Yes");
  await pollDialog.getByPlaceholder("Answer 2").fill("No");

  const pollResponse = page.waitForResponse(
    (response) =>
      response.request().method() === "POST" &&
      response.url().includes(`/channels/${server.generalChannelId}/polls`) &&
      response.status() === 200
  );
  await pollDialog.getByRole("button", { name: "Create poll" }).click();
  await pollResponse;
  await expect(pollDialog).toBeHidden();
  await expect(page.getByText(pollQuestion)).toBeVisible();

  await openCreatePollDialog(page, "Create proposal");
  const proposalDialog = page.getByRole("dialog", {
    name: "Create a New Proposal",
  });
  await proposalDialog.getByRole("combobox").click();
  await page.getByRole("option", { name: "General decision" }).click();
  await expect(
    proposalDialog.getByText(
      "Anonymous users can only create test proposals. Please register to create other proposal types."
    )
  ).toBeVisible();

  await proposalDialog.getByRole("combobox").click();
  await page.getByRole("option", { name: "Test" }).click();
  await proposalDialog
    .getByPlaceholder("Enter your proposal details...")
    .fill(proposalBody);
  await proposalDialog.getByRole("button", { name: "Next" }).click();

  const proposalResponse = page.waitForResponse(
    (response) =>
      response.request().method() === "POST" &&
      response.url().includes(`/channels/${server.generalChannelId}/polls`) &&
      response.status() === 200
  );
  await proposalDialog
    .getByRole("button", { name: "Create proposal" })
    .click();
  await proposalResponse;
  await expect(proposalDialog).toBeHidden();
  await expect(page.getByText(proposalBody)).toBeVisible();
});

async function openCreatePollDialog(
  page: Page,
  menuItemName: "Create poll" | "Create proposal"
) {
  await page
    .locator("form")
    .filter({ has: page.getByPlaceholder("Send a message...") })
    .getByRole("button")
    .first()
    .click();
  await page.getByRole("menuitem", { name: menuItemName }).click();
}

async function getDefaultServer(
  request: APIRequestContext,
  user: AuthenticatedUser
) {
  const response = await request.get("/api/servers/default", {
    headers: authorizationHeaders(user),
  });

  await expect(response).toBeOK();
  return ((await response.json()) as ServerResponse).server;
}

async function enableAnonymousUsers(
  request: APIRequestContext,
  user: AuthenticatedUser,
  serverId: string
) {
  const response = await request.put(`/api/servers/${serverId}/configs`, {
    headers: authorizationHeaders(user),
    data: { anonymousUsersEnabled: true },
  });

  await expect(response).toBeOK();
}

async function createInvite(
  request: APIRequestContext,
  user: AuthenticatedUser,
  serverId: string
) {
  const response = await request.post(`/api/servers/${serverId}/invites`, {
    headers: authorizationHeaders(user),
    data: {},
  });

  await expect(response).toBeOK();
  return ((await response.json()) as InviteResponse).invite.token;
}

function authorizationHeaders(user: AuthenticatedUser) {
  return {
    Authorization: `Bearer ${user.accessToken}`,
  };
}
