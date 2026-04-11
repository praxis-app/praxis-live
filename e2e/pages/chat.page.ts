import { expect, type Locator, type Page } from "@playwright/test";

export class ChatPage {
  constructor(private readonly page: Page) {}

  messageFeed(): Locator {
    return this.page.getByLabel("Message feed");
  }

  async goto() {
    await this.page.goto("/chat");
  }

  async sendMessage(message: string) {
    await this.page.getByRole("textbox", { name: "Message" }).fill(message);
    await this.page.getByRole("button", { name: "Send message" }).click();
  }

  async expectChannel(channelName: string) {
    await expect(this.page.getByRole("heading", { name: channelName })).toBeVisible();
  }

  async expectEmptyFeed() {
    await expect(this.page.getByText("No messages yet")).toBeVisible();
  }

  async expectMessage(message: string, author: string) {
    await expect(this.page.getByText(message)).toBeVisible();
    await expect(this.page.locator(".message-row").filter({ hasText: message })).toContainText(
      author,
    );
  }
}
