import { expect, type Page } from '@playwright/test';

import { ACCESS_TOKEN_KEY } from '../lib/data';
import type { TestUser } from '../lib/data';

export class NavigationPage {
  constructor(private readonly page: Page) {}

  async expectSignedInUser(user: TestUser) {
    await expect(this.page.getByTitle(user.name).first()).toBeVisible();
  }

  async expectAccessTokenPersisted() {
    await expect
      .poll(
        () =>
          this.page.evaluate(
            (storageKey) => window.localStorage.getItem(storageKey),
            ACCESS_TOKEN_KEY,
          ),
        {
          message: 'access token is persisted',
        },
      )
      .not.toBeNull();
  }

  async logOut() {
    await this.page.getByTitle(/.+/).click();
    await this.page.getByRole('menuitem', { name: 'Log out' }).click();
    await this.page.getByRole('button', { name: 'Log out' }).click();
    await expect(this.page).toHaveURL(/\/auth\/login\/?/);
  }
}
