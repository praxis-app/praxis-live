import { NotificationInbox } from '@/components/notifications/notification-inbox';
import { useNotifications } from '@/components/notifications/notification-context';
import { getNotificationTargetRoute } from '@/components/notifications/notification-target';
import { Button } from '@/components/ui/button';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { Sheet, SheetContent, SheetTrigger } from '@/components/ui/sheet';
import { useIsDesktop } from '@/hooks/use-is-desktop';
import { useServerData } from '@/hooks/use-server-data';
import { type NotificationRes } from '@/types/notification.types';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { LuBell } from 'react-icons/lu';
import { useNavigate } from 'react-router-dom';

export const NotificationBell = () => {
  const [isOpen, setIsOpen] = useState(false);
  const { t } = useTranslation();
  const navigate = useNavigate();
  const isDesktop = useIsDesktop();
  const { serverSlug } = useServerData();
  const { enabled, unreadCount, markRead } = useNotifications();

  if (!enabled || !serverSlug) return null;

  const selectNotification = (notification: NotificationRes) => {
    const route = getNotificationTargetRoute(notification, serverSlug);
    if (!route) {
      return;
    }
    if (!notification.readAt) {
      markRead(notification);
    }
    setIsOpen(false);
    void navigate(route.path, { state: route.state });
  };

  const trigger = (
    <Button
      type="button"
      variant="ghost"
      size="icon"
      className="relative"
      aria-label={t('notifications.actions.open', { count: unreadCount })}
      data-testid="notification-bell"
    >
      <LuBell className="size-5.5" />
      {unreadCount > 0 && (
        <span
          data-testid="notification-count"
          className="bg-primary text-primary-foreground ring-background absolute -top-0.5 -right-0.5 flex min-w-4.5 items-center justify-center rounded-full px-1 text-[10px] leading-4.5 font-semibold ring-2"
        >
          {unreadCount > 99 ? '99+' : unreadCount}
        </span>
      )}
    </Button>
  );

  if (isDesktop) {
    return (
      <Popover open={isOpen} onOpenChange={setIsOpen}>
        <PopoverTrigger asChild>{trigger}</PopoverTrigger>
        <PopoverContent
          align="end"
          className="flex max-h-[min(36rem,calc(100vh-5rem))] w-96 overflow-hidden p-0"
        >
          <NotificationInbox onSelect={selectNotification} />
        </PopoverContent>
      </Popover>
    );
  }

  return (
    <Sheet open={isOpen} onOpenChange={setIsOpen}>
      <SheetTrigger asChild>{trigger}</SheetTrigger>
      <SheetContent
        side="bottom"
        className="flex max-h-[82dvh] gap-0 rounded-t-xl p-0"
      >
        <NotificationInbox onSelect={selectNotification} />
      </SheetContent>
    </Sheet>
  );
};
