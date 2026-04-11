import { test } from "@playwright/test";

import { AuthPage } from "../pages/auth.page";
import { ChatPage } from "../pages/chat.page";
import { NavigationPage } from "../pages/navigation.page";
import { createTestUser } from "../support/test-data";

test("user can sign up from the landing page", async ({ page }) => {
  const user = createTestUser("signup");
  const auth = new AuthPage(page);
  const chat = new ChatPage(page);
  const navigation = new NavigationPage(page);

  await auth.gotoLanding();
  await auth.followSignupLink();
  await auth.signUp(user);

  await auth.expectSignedUp();
  await chat.expectChannel("general");
  await navigation.expectSignedInUser(user);
  await navigation.expectAccessTokenPersisted();
});
