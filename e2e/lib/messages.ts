import {
  expect,
  type APIRequestContext,
  type Locator,
  type Page,
} from '@playwright/test';
import { authorizationHeaders, type AuthenticatedUser } from './auth';

interface CreateMessagesOptions {
  request: APIRequestContext;
  user: AuthenticatedUser;
  serverId: string;
  channelId: string;
  bodies: string[];
  callId?: string;
}

export async function createMessages({
  request,
  user,
  serverId,
  channelId,
  bodies,
  callId,
}: CreateMessagesOptions) {
  const messagePath = callId
    ? `/api/servers/${serverId}/channels/${channelId}/calls/${callId}/messages`
    : `/api/servers/${serverId}/channels/${channelId}/messages`;

  for (const body of bodies) {
    const response = await request.post(messagePath, {
      headers: authorizationHeaders(user),
      data: { body },
    });
    await expect(response).toBeOK();
  }
}

/** Radix opens its context menu 700ms into a touch press. */
const LONG_PRESS_MS = 900;

/**
 * Drives a real touch press through CDP rather than synthetic pointer events,
 * so the browser's own long-press selection behaviour is exercised too.
 */
export async function longPressMessage(page: Page, message: Locator) {
  const box = await message.boundingBox();
  if (!box) {
    throw new Error('Expected the message to have a bounding box');
  }

  const touchPoints = [{ x: box.x + box.width / 2, y: box.y + box.height / 2 }];
  const session = await page.context().newCDPSession(page);

  await session.send('Input.dispatchTouchEvent', {
    type: 'touchStart',
    touchPoints,
  });
  await page.waitForTimeout(LONG_PRESS_MS);
  await session.send('Input.dispatchTouchEvent', {
    type: 'touchEnd',
    touchPoints: [],
  });
  await session.detach();
}

export function getSelectedText(page: Page) {
  return page.evaluate(() => window.getSelection()?.toString() ?? '');
}
