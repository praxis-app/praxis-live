import { expect, type APIRequestContext, type Locator } from '@playwright/test';
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

/**
 * Starts a touch press and returns a release callback. Radix opens its context
 * menu 700ms after a non-mouse `pointerdown`, and cancels on move, up, or
 * cancel, so the press must be held without moving.
 */
export async function pressAndHold(target: Locator) {
  const box = await target.boundingBox();
  expect(box).not.toBeNull();

  await target.dispatchEvent('pointerdown', {
    pointerType: 'touch',
    isPrimary: true,
    button: 0,
    clientX: box!.x + box!.width / 2,
    clientY: box!.y + box!.height / 2,
  });

  return () => target.dispatchEvent('pointerup', { pointerType: 'touch' });
}

export async function longPress(target: Locator) {
  const release = await pressAndHold(target);
  await target.page().waitForTimeout(900);
  await release();
}
