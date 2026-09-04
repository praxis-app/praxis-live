import { type NotificationRes } from '@/types/notification.types';
import { createContext, useContext } from 'react';

export interface NotificationContextValue {
  enabled: boolean;
  notifications: NotificationRes[];
  unreadCount: number;
  isPending: boolean;
  isError: boolean;
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  fetchNextPage: () => void;
  refetch: () => void;
  markRead: (notification: NotificationRes) => void;
  markUnread: (notification: NotificationRes) => void;
  deleteNotification: (notification: NotificationRes) => void;
  markAllRead: () => void;
  clearAll: () => void;
}

export const NotificationContext = createContext<
  NotificationContextValue | undefined
>(undefined);

export const useNotifications = () => {
  const context = useContext(NotificationContext);
  if (!context) {
    throw new Error('useNotifications must be used within NotificationProvider');
  }
  return context;
};
