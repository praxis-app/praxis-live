import { test } from "@playwright/test";
import { createAuthenticatedUser } from "../lib/auth-api";
import { createTestMessage, createTestUser } from "../lib/test-data";
import { ChatPage } from "../pages/chat.page";
import { NavigationPage } from "../pages/navigation.page";

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
