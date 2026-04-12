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
