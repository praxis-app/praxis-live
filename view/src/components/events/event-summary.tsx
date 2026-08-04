import { FormattedText } from '@/components/shared/formatted-text';
import { UserAvatar } from '@/components/users/user-avatar';
import { LazyLoadImage } from '@/components/images/lazy-load-image';
import {
  formatEventDateTime,
  formatEventDuration,
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
      className={cn(
        'h-36 w-full',
        embedded ? 'rounded-lg' : 'rounded-none',
      )}
    />
  );

  return (
    <div
      className={cn(
        'min-w-0',
        embedded
          ? 'px-1'
          : 'bg-card overflow-hidden rounded-xl border shadow-sm',
      )}
    >
      {coverPhotoElement && (
        <div className={cn(embedded && 'pb-4')}>{coverPhotoElement}</div>
      )}
      <div className={cn(!embedded && 'px-5 py-5')}>
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
    </div>
  );
};
