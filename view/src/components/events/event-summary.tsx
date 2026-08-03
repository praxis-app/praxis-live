import { Badge } from '@/components/ui/badge';
import {
  formatEventDateTime,
  formatEventDuration,
  getTimeZone,
} from '@/lib/event.utils';
import { type UserRes } from '@/types/user.types';
import { useTranslation } from 'react-i18next';
import {
  MdLink,
  MdLocationOn,
  MdPeople,
  MdSchedule,
  MdVideocam,
} from 'react-icons/md';

interface Props {
  name: string;
  description: string;
  startsAt: string;
  endsAt?: string | null;
  online: boolean;
  location?: string | null;
  externalLink?: string | null;
  hosts: UserRes[];
}

export const EventSummary = ({
  name,
  description,
  startsAt,
  endsAt,
  online,
  location,
  externalLink,
  hosts,
}: Props) => {
  const { t } = useTranslation();
  const duration = formatEventDuration(startsAt, endsAt);
  return (
    <div className="min-w-0 space-y-3">
      <div>
        <h3 className="font-semibold">{name}</h3>
        <p className="text-muted-foreground mt-1 text-sm whitespace-pre-wrap">
          {description}
        </p>
      </div>
      <div className="text-muted-foreground space-y-2 text-sm">
        <div className="flex items-start gap-2">
          <MdSchedule className="mt-0.5 size-4 shrink-0" />
          <span>
            {formatEventDateTime(startsAt, endsAt)} · {getTimeZone()}
            {duration ? ` · ${duration}` : ''}
          </span>
        </div>
        <div className="flex items-center gap-2">
          {online ? (
            <MdVideocam className="size-4" />
          ) : (
            <MdLocationOn className="size-4" />
          )}
          <span>{online ? t('events.labels.online') : location}</span>
        </div>
        <div className="flex items-center gap-2">
          <MdPeople className="size-4" />
          <span>
            {t('events.labels.hostedBy', {
              names: hosts
                .map((host) => host.displayName || host.name)
                .join(', '),
            })}
          </span>
        </div>
        {externalLink && (
          <a
            className="text-primary flex items-center gap-2 hover:underline"
            href={externalLink}
            target="_blank"
            rel="noreferrer"
          >
            <MdLink className="size-4" />
            {externalLink}
          </a>
        )}
      </div>
      <Badge variant="secondary">
        {online ? t('events.labels.online') : t('events.labels.inPerson')}
      </Badge>
    </div>
  );
};
