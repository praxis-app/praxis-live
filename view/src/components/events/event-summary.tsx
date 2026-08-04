import { FormattedText } from '@/components/shared/formatted-text';
import { UserAvatar } from '@/components/users/user-avatar';
import { LazyLoadImage } from '@/components/images/lazy-load-image';
import {
  formatEventDateTime,
  formatEventDuration,
  getTimeZone,
} from '@/lib/event.utils';
import { cn } from '@/lib/shared.utils';
import { type UserRes } from '@/types/user.types';
import { type ImageRes } from '@/types/image.types';
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
  coverPhoto?: ImageRes | null;
  coverPhotoFile?: File;
  channelId?: string;
  pollId?: string;
  eventId?: string;
  embedded?: boolean;
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
  coverPhoto,
  coverPhotoFile,
  channelId,
  pollId,
  eventId,
  embedded = false,
}: Props) => {
  const { t } = useTranslation();
  const duration = formatEventDuration(startsAt, endsAt);
  const eventType = online
    ? t('events.labels.online')
    : t('events.labels.inPerson');
  const coverPhotoSrc = coverPhotoFile
    ? URL.createObjectURL(coverPhotoFile)
    : undefined;
  const hasCoverPhoto =
    !!coverPhotoSrc || (!!coverPhoto && !coverPhoto.isPlaceholder);
  const coverPhotoElement = hasCoverPhoto && (
    <LazyLoadImage
      alt={t('images.labels.coverPhoto')}
      src={coverPhotoSrc}
      imageId={coverPhoto?.id}
      isPlaceholder={coverPhoto?.isPlaceholder}
      channelId={channelId}
      pollId={pollId}
      eventId={eventId}
      eventCoverPhoto={!!pollId}
      className="h-36 w-full rounded-lg"
    />
  );

  if (embedded) {
    return (
      <div className="min-w-0 px-1">
        {coverPhotoElement && <div className="pb-4">{coverPhotoElement}</div>}
        <div className="pb-5">
          <p className="text-event-accent text-sm leading-relaxed font-medium tracking-wide uppercase">
            {formatEventDateTime(startsAt, endsAt)}
          </p>
          <h3 className="mt-2 text-lg leading-tight font-medium tracking-tight sm:text-xl">
            {name}
          </h3>
        </div>

        <div className="text-muted-foreground space-y-0.5 pb-5 text-sm">
          <div className="flex min-h-6 min-w-0 items-center gap-2.5">
            <MdPeople className="text-muted-foreground size-4 shrink-0" />
            <div className="flex min-w-0 items-center gap-2">
              <div className="flex shrink-0 -space-x-2">
                {hosts.slice(0, 3).map((host) => {
                  const hostName = host.displayName || host.name;
                  return (
                    <UserAvatar
                      key={host.id}
                      userId={host.id}
                      name={hostName}
                      imageId={host.profilePicture?.id}
                      className="border-card size-6 border-2"
                      fallbackClassName="text-xs"
                    />
                  );
                })}
              </div>
              <span className="min-w-0 truncate">
                {t('events.labels.hostedBy', {
                  names: hosts
                    .map((host) => host.displayName || host.name)
                    .join(', '),
                })}
              </span>
            </div>
          </div>

          {duration && (
            <div className="flex min-h-6 min-w-0 items-center gap-2.5">
              <MdSchedule className="text-muted-foreground size-4 shrink-0" />
              <span>
                {t('events.labels.duration')}: {duration}
              </span>
            </div>
          )}

          <div className="flex min-h-6 min-w-0 items-center gap-2.5">
            {online ? (
              <MdVideocam className="text-muted-foreground size-4 shrink-0" />
            ) : (
              <MdLocationOn className="text-muted-foreground size-4 shrink-0" />
            )}
            <span className="min-w-0 truncate">
              {online ? eventType : location}
            </span>
          </div>

          {externalLink && (
            <a
              className="text-primary flex min-h-6 min-w-0 items-center gap-2.5 hover:underline"
              href={externalLink}
              target="_blank"
              rel="noreferrer"
            >
              <MdLink className="text-muted-foreground size-4 shrink-0" />
              <span className="truncate">{externalLink}</span>
            </a>
          )}
        </div>

        {description && (
          <div className="border-t pt-4">
            <p className="mb-2 text-base font-medium">
              {t('events.headers.whatToExpect')}
            </p>
            <FormattedText
              text={description}
              className="text-sm leading-relaxed"
            />
          </div>
        )}
      </div>
    );
  }

  return (
    <div
      className={cn(
        'min-w-0 overflow-hidden',
        !embedded && 'bg-card rounded-xl border shadow-sm',
      )}
    >
      {coverPhotoElement}
      <div className="border-b px-4 py-5 sm:px-5">
        <div className="flex min-w-0 items-start justify-between gap-3">
          <div className="min-w-0">
            <p className="text-event-accent text-xs leading-relaxed font-bold tracking-wide uppercase">
              {formatEventDateTime(startsAt, endsAt)} · {getTimeZone()}
            </p>
            <h3 className="mt-1.5 text-xl leading-tight font-semibold tracking-tight sm:text-2xl">
              {name}
            </h3>
          </div>
          <span className="text-muted-foreground flex shrink-0 items-center gap-1.5 pt-0.5 text-sm font-medium">
            {online ? (
              <MdVideocam className="size-3.5" />
            ) : (
              <MdLocationOn className="size-3.5" />
            )}
            {eventType}
          </span>
        </div>
      </div>

      <div className="grid gap-2.5 p-4 sm:grid-cols-2 sm:p-5">
        {duration && (
          <div className="bg-muted/45 flex min-w-0 items-center gap-3 rounded-lg p-3">
            <span className="bg-background text-muted-foreground flex size-8 shrink-0 items-center justify-center rounded-md border shadow-xs">
              <MdSchedule className="size-4" />
            </span>
            <div className="min-w-0">
              <p className="text-muted-foreground text-xs font-medium">
                {t('events.labels.duration')}
              </p>
              <p className="truncate text-sm font-medium">{duration}</p>
            </div>
          </div>
        )}

        <div className="bg-muted/45 flex min-w-0 items-center gap-3 rounded-lg p-3">
          <span className="bg-background text-muted-foreground flex size-8 shrink-0 items-center justify-center rounded-md border shadow-xs">
            {online ? (
              <MdVideocam className="size-4" />
            ) : (
              <MdLocationOn className="size-4" />
            )}
          </span>
          <div className="min-w-0">
            <p className="text-muted-foreground text-xs font-medium">
              {online
                ? t('events.labels.eventType')
                : t('events.labels.location')}
            </p>
            <p className="truncate text-sm font-medium">
              {online ? eventType : location}
            </p>
          </div>
        </div>

        <div className="bg-muted/45 flex min-w-0 items-center gap-3 rounded-lg p-3 sm:col-span-2">
          <span className="bg-background text-muted-foreground flex size-8 shrink-0 items-center justify-center rounded-md border shadow-xs">
            <MdPeople className="size-4" />
          </span>
          <div className="flex min-w-0 items-center gap-2">
            <div className="flex shrink-0 -space-x-2">
              {hosts.slice(0, 3).map((host) => {
                const hostName = host.displayName || host.name;
                return (
                  <UserAvatar
                    key={host.id}
                    userId={host.id}
                    name={hostName}
                    imageId={host.profilePicture?.id}
                    className="border-card size-7 border-2"
                    fallbackClassName="text-xs"
                  />
                );
              })}
            </div>
            <p className="text-muted-foreground min-w-0 truncate text-sm">
              {t('events.labels.hostedBy', {
                names: hosts
                  .map((host) => host.displayName || host.name)
                  .join(', '),
              })}
            </p>
          </div>
        </div>

        {externalLink && (
          <a
            className="bg-muted/45 text-primary hover:bg-muted focus-visible:ring-ring flex min-w-0 items-center gap-3 rounded-lg p-3 transition-colors focus-visible:ring-2 focus-visible:outline-none sm:col-span-2"
            href={externalLink}
            target="_blank"
            rel="noreferrer"
          >
            <span className="bg-background text-muted-foreground flex size-8 shrink-0 items-center justify-center rounded-md border shadow-xs">
              <MdLink className="size-4" />
            </span>
            <span className="min-w-0 truncate text-sm font-medium underline-offset-4 hover:underline">
              {externalLink}
            </span>
          </a>
        )}
      </div>

      {description && (
        <div className="bg-muted/20 border-t px-4 py-4 sm:px-5 sm:py-5">
          <p className="mb-2 text-xl font-normal">
            {t('events.headers.whatToExpect')}
          </p>
          <FormattedText
            text={description}
            className="text-sm leading-relaxed"
          />
        </div>
      )}
    </div>
  );
};
