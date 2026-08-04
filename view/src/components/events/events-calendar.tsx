import { type EventRes } from '@/types/event.types';
import { Calendar, type CalendarApi, type EventClickInfo } from 'fullcalendar';
import dayGridPlugin from 'fullcalendar/daygrid';
import timeGridPlugin from 'fullcalendar/timegrid';
import classicThemePlugin from 'fullcalendar/themes/classic';
import { useEffect, useRef } from 'react';
import { useNavigate } from 'react-router-dom';

interface Props {
  events: EventRes[];
  date: Date;
  view: CalendarView;
  onDateChange: (date: Date) => void;
  serverPath: string;
}

export type CalendarView = 'month' | 'week';

export const EventsCalendar = ({
  events,
  date,
  view,
  onDateChange,
  serverPath,
}: Props) => {
  const elementRef = useRef<HTMLDivElement>(null);
  const calendarRef = useRef<CalendarApi | null>(null);
  const eventElementsRef = useRef(new Map<string, Set<HTMLElement>>());
  const initialDateRef = useRef(date);
  const initialViewRef = useRef(view);
  const onDateChangeRef = useRef(onDateChange);
  const navigate = useNavigate();
  const navigateRef = useRef(navigate);

  useEffect(() => {
    onDateChangeRef.current = onDateChange;
    navigateRef.current = navigate;
  }, [navigate, onDateChange]);

  useEffect(() => {
    if (!elementRef.current) return;

    const calendar = new Calendar(elementRef.current, {
      plugins: [dayGridPlugin, timeGridPlugin, classicThemePlugin],
      initialView:
        initialViewRef.current === 'month' ? 'dayGridMonth' : 'timeGridWeek',
      initialDate: initialDateRef.current,
      headerToolbar: false,
      height: 'auto',
      fixedWeekCount: true,
      dayMaxEvents: 3,
      nowIndicator: true,
      scrollTime: '08:00:00',
      dayCellTopClass: 'events-calendar-day-top',
      dayCellTopInnerClass: 'events-calendar-day-number',
      eventClass: 'events-calendar-event',
      eventDidMount: ({ event, el }) => {
        const eventElements = eventElementsRef.current;
        const elements = eventElements.get(event.id) || new Set<HTMLElement>();
        elements.add(el);
        eventElements.set(event.id, elements);
      },
      eventWillUnmount: ({ event, el }) => {
        const eventElements = eventElementsRef.current;
        const elements = eventElements.get(event.id);
        elements?.delete(el);
        if (!elements?.size) eventElements.delete(event.id);
      },
      eventMouseEnter: ({ event }) => {
        eventElementsRef.current
          .get(event.id)
          ?.forEach((element) => element.classList.add('is-hovered'));
      },
      eventMouseLeave: ({ event }) => {
        eventElementsRef.current
          .get(event.id)
          ?.forEach((element) => element.classList.remove('is-hovered'));
      },
      displayEventTime: true,
      eventTimeFormat: {
        hour: 'numeric',
        minute: '2-digit',
        meridiem: 'narrow',
      },
      datesSet: ({ view }) => {
        onDateChangeRef.current(view.currentStart);
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
    const nextView = view === 'month' ? 'dayGridMonth' : 'timeGridWeek';
    if (calendar.view.type !== nextView) {
      calendar.changeView(nextView, date);
    } else if (calendar.getDate().toDateString() !== date.toDateString()) {
      calendar.gotoDate(date);
    }
    calendar.setOption(
      'height',
      view === 'week' ? 'calc(100vh - 12rem)' : 'auto',
    );
  }, [date, view]);

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
