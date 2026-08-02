import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { addDays, eventOverlapsRange } from '@/lib/event-date.utils';
import { type EventRes } from '@/types/event.types';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';

interface Props {
  days: Date[];
  events: EventRes[];
  month: Date;
  serverPath: string;
}

export const EventsCalendar = ({ days, events, month, serverPath }: Props) => {
  const { t } = useTranslation();
  const weekdays = Array.from({ length: 7 }, (_, index) =>
    new Intl.DateTimeFormat(undefined, { weekday: 'short' }).format(
      addDays(new Date(2026, 0, 4), index),
    ),
  );
  return (
    <div className="overflow-hidden rounded-xl border">
      <div className="bg-muted/40 grid grid-cols-7 border-b">
        {weekdays.map((day) => (
          <div key={day} className="p-2 text-center text-xs font-medium">
            {day}
          </div>
        ))}
      </div>
      <div className="grid grid-cols-7">
        {days.map((day) => {
          const nextDay = addDays(day, 1);
          const dayEvents = events.filter((event) =>
            eventOverlapsRange(event, day, nextDay),
          );
          return (
            <div
              key={day.toISOString()}
              className="min-h-28 border-r border-b p-1.5 last:border-r-0"
            >
              <div
                className={
                  day.getMonth() === month.getMonth()
                    ? 'text-sm'
                    : 'text-muted-foreground text-sm'
                }
              >
                {day.getDate()}
              </div>
              <div className="mt-1 space-y-1">
                {dayEvents.slice(0, 3).map((event) => (
                  <Button
                    key={event.id}
                    variant="secondary"
                    size="sm"
                    className="h-auto w-full justify-start truncate px-2 py-1 text-xs"
                    asChild
                  >
                    <Link to={`${serverPath}/events/${event.id}`}>
                      {event.name}
                    </Link>
                  </Button>
                ))}
                {dayEvents.length > 3 && (
                  <Badge variant="outline">
                    {t('events.labels.more', { count: dayEvents.length - 3 })}
                  </Badge>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
