import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { formatEventDateTime } from '@/lib/event-date.utils';
import { type EventRes } from '@/types/event.types';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';

interface Props {
  events: EventRes[];
  serverPath: string;
}

export const EventsAgenda = ({ events, serverPath }: Props) => {
  const { t } = useTranslation();
  return (
    <div className="space-y-3">
      {events.map((event) => (
        <Link
          key={event.id}
          to={`${serverPath}/events/${event.id}`}
          className="block"
        >
          <Card className="gap-2 py-4">
            <CardHeader>
              <CardTitle className="text-base">{event.name}</CardTitle>
            </CardHeader>
            <CardContent className="text-muted-foreground space-y-1 text-sm">
              <p>{formatEventDateTime(event.startsAt, event.endsAt)}</p>
              <p>{event.online ? t('events.labels.online') : event.location}</p>
              <p>
                {t('events.labels.attendanceCounts', {
                  going: event.goingCount,
                  interested: event.interestedCount,
                })}
              </p>
            </CardContent>
          </Card>
        </Link>
      ))}
    </div>
  );
};
