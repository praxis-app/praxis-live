import { expect, test } from '@playwright/test';
import {
  authorizationHeaders,
  createAuthenticatedUser,
  signUpViaApi,
} from '../lib/auth';
import { createTestUser } from '../lib/data';
import {
  makeProposalsRatifyWithOneAgreeVote,
  openCreateProposalDialog,
  selectRadixOption,
} from '../lib/polls';
import { getDefaultServer } from '../lib/servers';
import { ChatPage } from '../pages/chat.page';

type UserSummary = {
  id: string;
  name: string;
  displayName?: string | null;
};

type ProposedEvent = {
  name: string;
  description: string;
  startsAt: string;
  endsAt: string | null;
  online: boolean;
  location: string | null;
  externalLink: string | null;
  hosts: UserSummary[];
  createdEventId?: string | null;
};

type EventResponse = ProposedEvent & {
  id: string;
  sourcePollActionId: string | null;
  currentUserStatus: 'host' | 'interested' | 'going' | null;
  interestedCount: number;
  goingCount: number;
};

type EventDetailResponse = EventResponse & {
  interested: UserSummary[];
  going: UserSummary[];
};

type PollResponse = {
  poll: {
    id: string;
    action?: {
      id: string;
      actionType: string;
      event?: ProposedEvent;
    };
  };
};

const toDateTimeLocal = (date: Date) => {
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(
    date.getDate(),
  )}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
};

test('user can propose and ratify an online event with all details preserved', async ({
  context,
  page,
  request,
}) => {
  test.setTimeout(60_000);

  const proposer = await createAuthenticatedUser(
    request,
    context,
    createTestUser('event-proposer'),
  );
  const host = await signUpViaApi(request, createTestUser('event-host'));
  const server = await getDefaultServer(request, proposer);
  await makeProposalsRatifyWithOneAgreeVote(request, proposer, server.id);

  const eventName = `Online planning session ${proposer.user.suffix}`;
  const eventDescription = `Plan the next campaign phase ${proposer.user.suffix}.`;
  const proposalBody = `Schedule ${eventName}`;
  const externalLink = `https://example.com/events/${proposer.user.suffix}`;
  const startsAt = new Date();
  startsAt.setDate(startsAt.getDate() + 7);
  startsAt.setHours(18, 0, 0, 0);
  const endsAt = new Date(startsAt.getTime() + 90 * 60_000);

  const chat = new ChatPage(page);
  await chat.goto();
  await chat.expectChannel('general');

  await openCreateProposalDialog(page);
  const dialog = page.getByRole('dialog', { name: 'Create a New Proposal' });
  await selectRadixOption(dialog, page, 'Select an action type', 'Plan event');
  await dialog
    .getByPlaceholder('Enter your proposal details...')
    .fill(proposalBody);
  await dialog.getByRole('button', { name: 'Next' }).click();

  await dialog.getByLabel('Event name').fill(eventName);
  await dialog.getByLabel('Description').fill(eventDescription);
  await dialog.getByLabel('Starts').fill(toDateTimeLocal(startsAt));
  await dialog.getByLabel('Ends (optional)').fill(toDateTimeLocal(endsAt));
  await dialog.getByRole('switch', { name: 'Online' }).click();
  await dialog.getByLabel('Online event link (optional)').fill(externalLink);
  await dialog.getByPlaceholder('Search members...').fill(host.user.name);
  await dialog.getByText(host.user.name, { exact: true }).click();
  await dialog.getByRole('button', { name: 'Next' }).click();

  await expect(dialog.getByText(eventName, { exact: true })).toBeVisible();
  await expect(
    dialog.getByText(eventDescription, { exact: true }),
  ).toBeVisible();
  await expect(dialog.getByText(/1h 30m/)).toBeVisible();
  await expect(
    dialog.locator('[data-slot="badge"]').filter({ hasText: /^Online$/ }),
  ).toBeVisible();
  await expect(dialog.getByRole('link', { name: externalLink })).toBeVisible();
  await expect(
    dialog.getByText(`Hosted by ${host.user.name}`, { exact: true }),
  ).toBeVisible();

  const createProposalResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes(`/channels/${server.generalChannelId}/polls`) &&
      response.status() === 200,
  );
  await dialog.getByRole('button', { name: 'Create proposal' }).click();
  const createResponse = await createProposalResponse;
  const { poll } = (await createResponse.json()) as PollResponse;
  const action = poll.action;
  const proposedEvent = action?.event;

  expect(action?.actionType).toBe('plan-event');
  expect(proposedEvent).toBeDefined();
  if (!action || !proposedEvent) {
    throw new Error('Plan-event proposal response did not include its action.');
  }
  expect(proposedEvent).toMatchObject({
    name: eventName,
    description: eventDescription,
    online: true,
    location: null,
    externalLink,
  });
  expect(proposedEvent.hosts.map((user) => user.id)).toEqual([host.userId]);

  await expect(dialog).toBeHidden();
  const proposal = page.getByRole('article', {
    name: `Majority Vote Proposal: ${proposalBody}`,
  });
  await expect(proposal).toBeVisible();
  await proposal
    .getByRole('button', { name: `Planned event: ${eventName}` })
    .click();
  await expect(proposal.getByText(eventName, { exact: true })).toBeVisible();
  await expect(
    proposal.getByText(eventDescription, { exact: true }),
  ).toBeVisible();
  await expect(proposal.getByText(/1h 30m/)).toBeVisible();
  await expect(
    proposal.getByRole('link', { name: externalLink }),
  ).toBeVisible();
  await expect(
    proposal.getByText(`Hosted by ${host.user.name}`, { exact: true }),
  ).toBeVisible();
  await expect(proposal.getByRole('link', { name: 'View event' })).toHaveCount(
    0,
  );

  const voteResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes(`/polls/${poll.id}/votes`) &&
      response.status() === 200,
  );
  await proposal.getByRole('button', { name: 'Agree', exact: true }).click();
  await voteResponse;
  await expect(proposal.getByText('Ratified', { exact: true })).toBeVisible();

  const eventsResponse = await request.get(`/api/servers/${server.id}/events`, {
    headers: authorizationHeaders(proposer),
    params: {
      from: new Date(startsAt.getTime() - 24 * 60 * 60_000).toISOString(),
      to: new Date(endsAt.getTime() + 24 * 60 * 60_000).toISOString(),
    },
  });
  await expect(eventsResponse).toBeOK();
  const events = ((await eventsResponse.json()) as { events: EventResponse[] })
    .events;
  const createdEvents = events.filter(
    (event) => event.sourcePollActionId === action.id,
  );
  expect(createdEvents).toHaveLength(1);
  const createdEvent = createdEvents[0];
  expect(createdEvent).toMatchObject({
    name: proposedEvent.name,
    description: proposedEvent.description,
    startsAt: proposedEvent.startsAt,
    endsAt: proposedEvent.endsAt,
    online: proposedEvent.online,
    location: proposedEvent.location,
    externalLink: proposedEvent.externalLink,
    currentUserStatus: null,
    interestedCount: 0,
    goingCount: 0,
  });
  expect(createdEvent.hosts.map((user) => user.id)).toEqual([host.userId]);

  const viewEventLink = proposal.getByRole('link', { name: 'View event' });
  await expect(viewEventLink).toHaveAttribute(
    'href',
    `/s/${server.slug}/events/${createdEvent.id}`,
  );
  const detailResponsePromise = page.waitForResponse(
    (response) =>
      response.request().method() === 'GET' &&
      response.url().includes(`/events/${createdEvent.id}`) &&
      response.status() === 200,
  );
  await viewEventLink.click();
  const detailResponse = await detailResponsePromise;
  const initialDetail = (
    (await detailResponse.json()) as {
      event: EventDetailResponse;
    }
  ).event;
  expect(initialDetail).toMatchObject(createdEvent);
  expect(initialDetail.hosts.map((user) => user.id)).toEqual([host.userId]);

  await expect(
    page.getByText(eventName, { exact: true }).first(),
  ).toBeVisible();
  await expect(page.getByText(eventDescription, { exact: true })).toBeVisible();
  await expect(page.getByText(/1h 30m/)).toBeVisible();
  await expect(page.getByRole('link', { name: externalLink })).toBeVisible();
  await expect(
    page.getByText(`Hosted by ${host.user.name}`, { exact: true }),
  ).toBeVisible();

  const interestedResponsePromise = page.waitForResponse(
    (response) =>
      response.request().method() === 'PUT' &&
      response.url().includes(`/events/${createdEvent.id}/rsvp`) &&
      response.status() === 200,
  );
  await page.getByRole('button', { name: 'Interested · 0' }).click();
  const interestedResponse = await interestedResponsePromise;
  const interestedEvent = (
    (await interestedResponse.json()) as {
      event: EventDetailResponse;
    }
  ).event;
  expect(interestedEvent.currentUserStatus).toBe('interested');
  expect(interestedEvent.interested.map((user) => user.id)).toContain(
    proposer.userId,
  );
  await expect(
    page.getByRole('button', { name: 'Interested · 1' }),
  ).toBeVisible();

  const goingResponsePromise = page.waitForResponse(
    (response) =>
      response.request().method() === 'PUT' &&
      response.url().includes(`/events/${createdEvent.id}/rsvp`) &&
      response.status() === 200,
  );
  await page.getByRole('button', { name: 'Going · 0' }).click();
  const goingResponse = await goingResponsePromise;
  const goingEvent = (
    (await goingResponse.json()) as {
      event: EventDetailResponse;
    }
  ).event;
  expect(goingEvent.currentUserStatus).toBe('going');
  expect(goingEvent.interested.map((user) => user.id)).not.toContain(
    proposer.userId,
  );
  expect(goingEvent.going.map((user) => user.id)).toContain(proposer.userId);
  await expect(
    page.getByRole('button', { name: 'Interested · 0' }),
  ).toBeVisible();
  await expect(page.getByRole('button', { name: 'Going · 1' })).toBeVisible();

  const calendarResponsePromise = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return (
      response.request().method() === 'GET' &&
      url.pathname === `/api/servers/${server.id}/events` &&
      response.status() === 200
    );
  });
  await page.getByRole('link', { name: 'Events', exact: true }).click();
  await calendarResponsePromise;
  const calendarEvent = page
    .getByRole('grid')
    .getByText(eventName, { exact: true });
  await expect(calendarEvent).toBeVisible();
  await calendarEvent.click();
  await expect(page).toHaveURL(`/s/${server.slug}/events/${createdEvent.id}`);
  await expect(page.getByRole('button', { name: 'Going · 1' })).toBeVisible();
});
