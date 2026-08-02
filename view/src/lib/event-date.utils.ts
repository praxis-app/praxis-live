export const startOfDay = (date: Date) =>
  new Date(date.getFullYear(), date.getMonth(), date.getDate());

export const addDays = (date: Date, days: number) => {
  const result = new Date(date);
  result.setDate(result.getDate() + days);
  return result;
};

export const startOfWeek = (date: Date) =>
  addDays(startOfDay(date), -startOfDay(date).getDay());

export const startOfMonth = (date: Date) =>
  new Date(date.getFullYear(), date.getMonth(), 1);

export const addMonths = (date: Date, months: number) =>
  new Date(date.getFullYear(), date.getMonth() + months, 1);

export const eventEnd = (event: { startsAt: string; endsAt?: string | null }) =>
  new Date(event.endsAt || event.startsAt);

export const eventOverlapsRange = (
  event: { startsAt: string; endsAt?: string | null },
  from: Date,
  to: Date,
) => new Date(event.startsAt) < to && eventEnd(event) > from;

export const formatEventDateTime = (
  startsAt: string,
  endsAt?: string | null,
) => {
  const start = new Date(startsAt);
  const end = endsAt ? new Date(endsAt) : null;
  const date = new Intl.DateTimeFormat(undefined, {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  }).format(start);
  const time = new Intl.DateTimeFormat(undefined, {
    hour: 'numeric',
    minute: '2-digit',
  });
  if (!end) return `${date}, ${time.format(start)}`;
  const sameDay = start.toDateString() === end.toDateString();
  return sameDay
    ? `${date}, ${time.format(start)}–${time.format(end)}`
    : `${date}, ${time.format(start)} – ${new Intl.DateTimeFormat(undefined, {
        month: 'short',
        day: 'numeric',
        hour: 'numeric',
        minute: '2-digit',
      }).format(end)}`;
};

export const getTimeZone = () =>
  Intl.DateTimeFormat().resolvedOptions().timeZone;

export const formatEventDuration = (
  startsAt: string,
  endsAt?: string | null,
) => {
  if (!endsAt) return null;
  const minutes = Math.round(
    (new Date(endsAt).getTime() - new Date(startsAt).getTime()) / 60_000,
  );
  const hours = Math.floor(minutes / 60);
  const remainder = minutes % 60;
  return [hours ? `${hours}h` : '', remainder ? `${remainder}m` : '']
    .filter(Boolean)
    .join(' ');
};
