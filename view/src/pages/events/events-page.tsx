import { DecisionsPanel } from '@/components/decisions/decisions-panel';
import {
  type CalendarView,
  EventsCalendar,
} from '@/components/events/events-calendar';
import { EventsList } from '@/components/events/events-list';
import { LeftNavDesktop } from '@/components/nav/left-nav-desktop';
import { TopNav } from '@/components/nav/top-nav';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { LocalStorageKeys } from '@/constants/shared.constants';
import { useEventsQuery } from '@/hooks/events/use-events-query';
import { useAuthData } from '@/hooks/use-auth-data';
import { useIsDesktop } from '@/hooks/use-is-desktop';
import { useServerData } from '@/hooks/use-server-data';
import {
  addDays,
  addMonths,
  parseDateValue,
  startOfMonth,
  startOfWeek,
  toDateValue,
} from '@/lib/event.utils';
import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { LuCalendarDays, LuChevronLeft, LuChevronRight } from 'react-icons/lu';
import { useSearchParams } from 'react-router-dom';

type EventView = CalendarView | 'list';

const LARGE_DESKTOP_MEDIA_QUERY = '(min-width: 1200px)';
const EVENT_VIEWS: EventView[] = ['list', 'month', 'week'];

const isEventView = (value: string | null): value is EventView =>
  EVENT_VIEWS.some((view) => view === value);

const getDateFromParam = (value: string | null) => {
  const date = value && parseDateValue(value);
  return date && toDateValue(date) === value ? date : new Date();
};

const getDefaultDecisionsPanelOpen = () => {
  const storedPreference = localStorage.getItem(
    LocalStorageKeys.DecisionsPanelOpen,
  );
  return (
    storedPreference === 'true' ||
    (storedPreference !== 'false' &&
      window.matchMedia(LARGE_DESKTOP_MEDIA_QUERY).matches)
  );
};

export const EventsPage = () => {
  const [isDecisionsPanelOpen, setIsDecisionsPanelOpen] = useState(
    getDefaultDecisionsPanelOpen,
  );

  const { me } = useAuthData();
  const { serverId, serverPath } = useServerData();

  const { t } = useTranslation();
  const isDesktop = useIsDesktop();
  const [searchParams, setSearchParams] = useSearchParams();

  const viewParam = searchParams.get('view');
  const view = isEventView(viewParam) ? viewParam : 'list';
  const date = getDateFromParam(searchParams.get('date'));

  useEffect(() => {
    setIsDecisionsPanelOpen(getDefaultDecisionsPanelOpen());
  }, [serverId]);

  const closeDecisionsPanel = () => {
    localStorage.setItem(LocalStorageKeys.DecisionsPanelOpen, 'false');
    setIsDecisionsPanelOpen(false);
  };

  const toggleDecisionsPanel = () => {
    const nextIsOpen = !isDecisionsPanelOpen;
    localStorage.setItem(
      LocalStorageKeys.DecisionsPanelOpen,
      String(nextIsOpen),
    );
    setIsDecisionsPanelOpen(nextIsOpen);
  };

  const setEventState = (nextView: EventView, nextDate: Date) => {
    const nextSearchParams = new URLSearchParams(searchParams);
    nextSearchParams.set('view', nextView);
    nextSearchParams.set('date', toDateValue(nextDate));
    setSearchParams(nextSearchParams);
  };

  const { from, to } = useMemo(() => {
    if (view === 'week') {
      const weekStart = startOfWeek(date);
      return { from: weekStart, to: addDays(weekStart, 7) };
    }
    if (view === 'list') {
      const monthStart = startOfMonth(date);
      return { from: monthStart, to: addMonths(monthStart, 1) };
    }
    const gridStart = startOfWeek(startOfMonth(date));
    return { from: gridStart, to: addDays(gridStart, 42) };
  }, [date, view]);

  const eventsQuery = useEventsQuery(serverId, {
    from: from.toISOString(),
    to: to.toISOString(),
  });

  const events = eventsQuery.data?.events || [];

  const dateLabel = useMemo(() => {
    if (view !== 'week') {
      return new Intl.DateTimeFormat(undefined, {
        month: 'long',
        year: 'numeric',
      }).format(date);
    }
    const weekStart = startOfWeek(date);
    const weekEnd = addDays(weekStart, 6);
    return new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    }).formatRange(weekStart, weekEnd);
  }, [date, view]);

  const navigateDate = (direction: -1 | 1) => {
    setEventState(
      view,
      view === 'week'
        ? addDays(date, direction * 7)
        : addMonths(date, direction),
    );
  };

  const showEventsEmptyMessage =
    !eventsQuery.isLoading && !eventsQuery.isError && events.length === 0;

  return (
    <div className="fixed inset-0 flex">
      {isDesktop && <LeftNavDesktop me={me} />}

      <div className="flex min-w-0 flex-1 flex-col">
        <TopNav
          header={t('events.title')}
          subheader={dateLabel}
          showSearch={isDesktop}
          hideBackButtonOnDesktop
          isDecisionsPanelOpen={isDecisionsPanelOpen}
          onToggleDecisionsPanel={toggleDecisionsPanel}
        />

        <main className="flex-1 overflow-y-auto p-3 sm:p-6">
          <div className="mx-auto max-w-7xl space-y-6">
            <div className="space-y-3 sm:flex sm:items-center sm:justify-between sm:gap-3 sm:space-y-0">
              <div className="grid grid-cols-[auto_1fr_auto] gap-2 sm:hidden">
                <Button
                  variant="outline"
                  size="icon"
                  aria-label={t('events.actions.previous')}
                  onClick={() => navigateDate(-1)}
                >
                  <LuChevronLeft className="size-5" />
                </Button>
                <Button
                  variant="outline"
                  className="min-w-0"
                  onClick={() => setEventState(view, new Date())}
                >
                  <LuCalendarDays className="size-4 sm:hidden" />
                  {t('events.actions.today')}
                </Button>
                <Button
                  variant="outline"
                  size="icon"
                  aria-label={t('events.actions.next')}
                  onClick={() => navigateDate(1)}
                >
                  <LuChevronRight className="size-5" />
                </Button>
              </div>

              <div className="hidden items-center gap-2 sm:flex">
                <div className="flex">
                  <Button
                    variant="outline"
                    size="icon"
                    className="rounded-r-none"
                    aria-label={t('events.actions.previous')}
                    onClick={() => navigateDate(-1)}
                  >
                    <LuChevronLeft className="size-5" />
                  </Button>
                  <Button
                    variant="outline"
                    size="icon"
                    className="-ml-px rounded-l-none"
                    aria-label={t('events.actions.next')}
                    onClick={() => navigateDate(1)}
                  >
                    <LuChevronRight className="size-5" />
                  </Button>
                </div>
                <Button
                  variant="outline"
                  onClick={() => setEventState(view, new Date())}
                >
                  {t('events.actions.today')}
                </Button>
              </div>

              <div className="bg-muted grid grid-cols-3 gap-1 rounded-xl p-1 sm:flex sm:bg-transparent sm:p-0">
                {EVENT_VIEWS.map((value) => (
                  <Button
                    key={value}
                    size="sm"
                    variant={view === value ? 'secondary' : 'ghost'}
                    className={
                      view === value
                        ? 'bg-background hover:bg-background sm:bg-foreground sm:text-background sm:hover:bg-foreground/90 shadow-xs'
                        : 'text-muted-foreground'
                    }
                    aria-pressed={view === value}
                    onClick={() => setEventState(value, date)}
                  >
                    {t(`events.views.${value}`)}
                  </Button>
                ))}
              </div>
            </div>
            {eventsQuery.isLoading && (
              <div className="space-y-3">
                <Skeleton className="h-24 w-full" />
                <Skeleton className="h-64 w-full" />
              </div>
            )}
            {eventsQuery.isError && (
              <p className="text-destructive">{t('events.errors.load')}</p>
            )}
            {showEventsEmptyMessage && (
              <div className="rounded-xl border border-dashed p-12 text-center">
                <h2 className="font-semibold">{t('events.empty.title')}</h2>
                <p className="text-muted-foreground mt-1 text-sm">
                  {t('events.empty.description')}
                </p>
              </div>
            )}
            {!eventsQuery.isLoading &&
              !eventsQuery.isError &&
              (view === 'list' ? (
                events.length > 0 && (
                  <EventsList events={events} serverPath={serverPath} />
                )
              ) : (
                <EventsCalendar
                  events={events}
                  date={date}
                  view={view}
                  serverPath={serverPath}
                />
              ))}
          </div>
        </main>
      </div>
      {isDesktop && (
        <DecisionsPanel
          isOpen={isDecisionsPanelOpen}
          onClose={closeDecisionsPanel}
        />
      )}
    </div>
  );
};
