import { expect, type Page } from "@playwright/test";

import { ACCESS_TOKEN_KEY } from "../support/test-data";
import type { TestUser } from "../support/test-data";

export class NavigationPage {
  constructor(private readonly page: Page) {}

  async expectSignedInUser(user: TestUser) {
    await expect(this.page.locator(".sidebar__user-name")).toHaveText(user.name);
    await expect(this.page.getByText(user.email)).toBeVisible();
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
          message: "access token is persisted",
        },
      )
      .not.toBeNull();
  }

  async logOut() {
    await this.page.getByRole("button", { name: "Log out" }).click();
    await expect(this.page).toHaveURL(/\/login\/?/);
  }
}
