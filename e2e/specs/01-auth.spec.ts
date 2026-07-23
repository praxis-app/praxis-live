import { test } from '@playwright/test';
import { setupAnonymousSession } from '../lib/auth';
import { createTestUser } from '../lib/data';
import { AuthPage } from '../pages/auth.page';
import { ChatPage } from '../pages/chat.page';
import { NavigationPage } from '../pages/navigation.page';

test('user can sign up from the landing page', async ({ page }) => {
  const user = createTestUser('signup');
  const auth = new AuthPage(page);
  const chat = new ChatPage(page);
  const navigation = new NavigationPage(page);

  await auth.gotoLanding();
  await auth.followSignupLink();
  await auth.signUp(user);

  await auth.expectSignedUp();
  await chat.expectChannel('general');
  await navigation.expectSignedInUser(user);
  await navigation.expectAccessTokenPersisted();
});

test('anonymous user can register from the landing page', async ({
  context,
  page,
  request,
}) => {
  await setupAnonymousSession(request, context, 'anon-upgrade');
  const user = createTestUser('anon-upgrade');
  const auth = new AuthPage(page);
  const chat = new ChatPage(page);
  const navigation = new NavigationPage(page);

  await auth.gotoLanding();
  await auth.followSignupLink();

  const upgradeResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'PUT' &&
      response.url().endsWith('/api/auth/anon') &&
      response.status() === 204,
  );
  await auth.signUp(user);
  await upgradeResponse;

  await auth.expectSignedUp();
  await chat.expectChannel('general');
  await navigation.expectSignedInUser(user);
  await navigation.expectAccessTokenPersisted();
});
