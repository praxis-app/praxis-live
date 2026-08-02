import { type UserRes } from './user.types';

export type EventRsvpStatus = 'interested' | 'going';
export type EventAttendeeStatus = 'host' | EventRsvpStatus;

export interface EventRes {
  id: string;
  name: string;
  description: string;
  startsAt: string;
  endsAt: string | null;
  online: boolean;
  location: string | null;
  externalLink: string | null;
  hosts: UserRes[];
  goingCount: number;
  interestedCount: number;
  currentUserStatus: EventAttendeeStatus | null;
  sourcePollActionId: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface EventDetailRes extends EventRes {
  going: UserRes[];
  interested: UserRes[];
}

export interface EventsQuery {
  from: string;
  to: string;
  online?: boolean;
}

export interface EventRsvpReq {
  status: EventRsvpStatus;
}
