import { startOfMonth } from '@/lib/event-date.utils';
import { type EventRes } from '@/types/event.types';
import { Calendar, type CalendarApi, type EventClickInfo } from 'fullcalendar';
import dayGridPlugin from 'fullcalendar/daygrid';
import classicThemePlugin from 'fullcalendar/themes/classic';
import { useEffect, useRef } from 'react';
import { useNavigate } from 'react-router-dom';

interface Props {
  events: EventRes[];
  month: Date;
  onMonthChange: (month: Date) => void;
  serverPath: string;
}

export const EventsCalendar = ({
  events,
  month,
  onMonthChange,
  serverPath,
}: Props) => {
  const elementRef = useRef<HTMLDivElement>(null);
  const calendarRef = useRef<CalendarApi | null>(null);
  const initialMonthRef = useRef(month);
  const onMonthChangeRef = useRef(onMonthChange);
  const navigate = useNavigate();
  const navigateRef = useRef(navigate);

  useEffect(() => {
    onMonthChangeRef.current = onMonthChange;
    navigateRef.current = navigate;
  }, [navigate, onMonthChange]);

  useEffect(() => {
    if (!elementRef.current) return;

    const calendar = new Calendar(elementRef.current, {
      plugins: [dayGridPlugin, classicThemePlugin],
      initialView: 'dayGridMonth',
      initialDate: initialMonthRef.current,
      headerToolbar: false,
      height: 'auto',
      fixedWeekCount: true,
      dayMaxEvents: 3,
      dayCellTopClass: 'events-calendar-day-top',
      dayCellTopInnerClass: 'events-calendar-day-number',
      displayEventTime: true,
      eventTimeFormat: {
        hour: 'numeric',
        minute: '2-digit',
        meridiem: 'narrow',
      },
      datesSet: ({ view }) => {
        onMonthChangeRef.current(startOfMonth(view.currentStart));
      },
      eventClick: (info: EventClickInfo) => {
        info.jsEvent.preventDefault();
        navigateRef.current(`${serverPath}/events/${info.event.id}`);
      },
    });
    calendar.render();
    calendarRef.current = calendar;

    return () => {
      calendar.destroy();
      calendarRef.current = null;
    };
  }, [serverPath]);

  useEffect(() => {
    const calendar = calendarRef.current;
    if (!calendar) return;
    if (
      calendar.getDate().getFullYear() !== month.getFullYear() ||
      calendar.getDate().getMonth() !== month.getMonth()
    ) {
      calendar.gotoDate(month);
    }
  }, [month]);

  useEffect(() => {
    const calendar = calendarRef.current;
    if (!calendar) return;
    calendar.removeAllEvents();
    calendar.addEventSource(
      events.map((event) => ({
        id: event.id,
        title: event.name,
        start: event.startsAt,
        end: event.endsAt || undefined,
      })),
    );
  }, [events]);

  return <div ref={elementRef} className="events-calendar" />;
};
