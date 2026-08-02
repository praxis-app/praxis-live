import { EventsAgenda } from '@/components/events/events-agenda';
import { EventsCalendar } from '@/components/events/events-calendar';
import { LeftNavDesktop } from '@/components/nav/left-nav-desktop';
import { TopNav } from '@/components/nav/top-nav';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { useAuthData } from '@/hooks/use-auth-data';
import { useEventsQuery } from '@/hooks/events/use-events-query';
import { useIsDesktop } from '@/hooks/use-is-desktop';
import { useServerData } from '@/hooks/use-server-data';
import {
  addDays,
  addMonths,
  eventEnd,
  eventOverlapsRange,
  startOfMonth,
  startOfWeek,
} from '@/lib/event-date.utils';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

type EventFilter = 'upcoming' | 'this-week' | 'online' | 'past';

export const EventsPage = () => {
  const [month, setMonth] = useState(startOfMonth(new Date()));
  const [filter, setFilter] = useState<EventFilter>('upcoming');
  const { t } = useTranslation();
  const isDesktop = useIsDesktop();
  const { me } = useAuthData();
  const { serverId, serverPath } = useServerData();
  const days = useMemo(() => {
    const first = startOfWeek(month);
    return Array.from({ length: 42 }, (_, index) => addDays(first, index));
  }, [month]);
  const from = days[0];
  const to = addDays(days[days.length - 1], 1);
  const query = useEventsQuery(serverId, {
    from: from.toISOString(),
    to: to.toISOString(),
    online: filter === 'online' ? true : undefined,
  });
  const events = useMemo(() => {
    const now = new Date();
    const weekStart = startOfWeek(now);
    const weekEnd = addDays(weekStart, 7);
    return (query.data?.events || []).filter((event) => {
      if (filter === 'past') return eventEnd(event) < now;
      if (filter === 'this-week')
        return eventOverlapsRange(event, weekStart, weekEnd);
      if (filter === 'online') return event.online && eventEnd(event) >= now;
      return eventEnd(event) >= now;
    });
  }, [filter, query.data?.events]);
  const monthLabel = new Intl.DateTimeFormat(undefined, {
    month: 'long',
    year: 'numeric',
  }).format(month);

  return (
    <div className="fixed inset-0 flex">
      {isDesktop && <LeftNavDesktop me={me} />}
      <div className="flex min-w-0 flex-1 flex-col">
        <TopNav
          header={t('events.title')}
          subheader={monthLabel}
          showSearch={false}
        />
        <main className="flex-1 overflow-y-auto p-4 sm:p-6">
          <div className="mx-auto max-w-7xl space-y-4">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  onClick={() => setMonth(addMonths(month, -1))}
                >
                  {t('events.actions.previous')}
                </Button>
                <Button
                  variant="outline"
                  onClick={() => setMonth(startOfMonth(new Date()))}
                >
                  {t('events.actions.today')}
                </Button>
                <Button
                  variant="outline"
                  onClick={() => setMonth(addMonths(month, 1))}
                >
                  {t('events.actions.next')}
                </Button>
              </div>
              <div className="flex flex-wrap gap-1">
                {(
                  ['upcoming', 'this-week', 'online', 'past'] as EventFilter[]
                ).map((value) => (
                  <Button
                    key={value}
                    size="sm"
                    variant={filter === value ? 'default' : 'ghost'}
                    onClick={() => setFilter(value)}
                  >
                    {t(`events.filters.${value}`)}
                  </Button>
                ))}
              </div>
            </div>
            {query.isLoading && (
              <div className="space-y-3">
                <Skeleton className="h-24 w-full" />
                <Skeleton className="h-64 w-full" />
              </div>
            )}
            {query.isError && (
              <p className="text-destructive">{t('events.errors.load')}</p>
            )}
            {!query.isLoading &&
              !query.isError &&
              events.length === 0 &&
              !isDesktop && (
                <div className="rounded-xl border border-dashed p-12 text-center">
                  <h2 className="font-semibold">{t('events.empty.title')}</h2>
                  <p className="text-muted-foreground mt-1 text-sm">
                    {t('events.empty.description')}
                  </p>
                </div>
              )}
            {!query.isLoading &&
              !query.isError &&
              (isDesktop ? (
                <EventsCalendar
                  days={days}
                  events={events}
                  month={month}
                  serverPath={serverPath}
                />
              ) : events.length > 0 ? (
                <EventsAgenda events={events} serverPath={serverPath} />
              ) : null)}
          </div>
        </main>
      </div>
    </div>
  );
};
