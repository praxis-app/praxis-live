import { EventSummary } from '@/components/events/event-summary';
import { LeftNavDesktop } from '@/components/nav/left-nav-desktop';
import { TopNav } from '@/components/nav/top-nav';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { UserAvatar } from '@/components/users/user-avatar';
import { useAuthData } from '@/hooks/use-auth-data';
import { useEventQuery } from '@/hooks/events/use-event-query';
import { useEventRsvp } from '@/hooks/events/use-event-rsvp';
import { useIsDesktop } from '@/hooks/use-is-desktop';
import { useServerData } from '@/hooks/use-server-data';
import { type UserRes } from '@/types/user.types';
import { useTranslation } from 'react-i18next';
import { useNavigate, useParams } from 'react-router-dom';

const Attendees = ({ users }: { users: UserRes[] }) => (
  <div className="flex flex-wrap gap-3">
    {users.map((user) => (
      <div key={user.id} className="flex items-center gap-2">
        <UserAvatar
          userId={user.id}
          name={user.displayName || user.name}
          imageId={user.profilePicture?.id}
          className="size-7"
        />
        <span className="text-sm">{user.displayName || user.name}</span>
      </div>
    ))}
  </div>
);

export const EventDetailPage = () => {
  const { eventId } = useParams();
  const { t } = useTranslation();
  const navigate = useNavigate();
  const isDesktop = useIsDesktop();
  const { me } = useAuthData();
  const { serverId, serverPath } = useServerData();
  const query = useEventQuery(serverId, eventId);
  const rsvp = useEventRsvp(serverId, eventId);
  const event = query.data?.event;
  const setRsvp = (status: 'interested' | 'going') =>
    rsvp.mutate(event?.currentUserStatus === status ? null : status);

  return (
    <div className="fixed inset-0 flex">
      {isDesktop && <LeftNavDesktop me={me} />}
      <div className="flex min-w-0 flex-1 flex-col">
        <TopNav
          header={event?.name || t('events.title')}
          onBackClick={() => navigate(`${serverPath}/events`)}
          showSearch={false}
        />
        <main className="flex-1 overflow-y-auto p-4 sm:p-6">
          <div className="mx-auto max-w-3xl space-y-5">
            {query.isLoading && <Skeleton className="h-80 w-full" />}
            {query.isError && (
              <p className="text-destructive">
                {t('events.errors.loadDetail')}
              </p>
            )}
            {event && (
              <>
                <Card>
                  <CardContent>
                    <EventSummary {...event} />
                  </CardContent>
                </Card>
                <div className="flex gap-2">
                  <Button
                    variant={
                      event.currentUserStatus === 'interested'
                        ? 'default'
                        : 'outline'
                    }
                    disabled={
                      event.currentUserStatus === 'host' || rsvp.isPending
                    }
                    onClick={() => setRsvp('interested')}
                  >
                    {t('events.actions.interested')} · {event.interestedCount}
                  </Button>
                  <Button
                    variant={
                      event.currentUserStatus === 'going'
                        ? 'default'
                        : 'outline'
                    }
                    disabled={
                      event.currentUserStatus === 'host' || rsvp.isPending
                    }
                    onClick={() => setRsvp('going')}
                  >
                    {t('events.actions.going')} · {event.goingCount}
                  </Button>
                </div>
                {event.currentUserStatus === 'host' && (
                  <p className="text-muted-foreground text-sm">
                    {t('events.labels.hostRsvp')}
                  </p>
                )}
                {rsvp.isError && (
                  <p className="text-destructive text-sm">
                    {t('events.errors.rsvp')}
                  </p>
                )}
                <Card>
                  <CardHeader>
                    <CardTitle>{t('events.labels.going')}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    {event.going.length ? (
                      <Attendees users={event.going} />
                    ) : (
                      <p className="text-muted-foreground text-sm">
                        {t('events.empty.attendees')}
                      </p>
                    )}
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader>
                    <CardTitle>{t('events.labels.interested')}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    {event.interested.length ? (
                      <Attendees users={event.interested} />
                    ) : (
                      <p className="text-muted-foreground text-sm">
                        {t('events.empty.attendees')}
                      </p>
                    )}
                  </CardContent>
                </Card>
              </>
            )}
          </div>
        </main>
      </div>
    </div>
  );
};
