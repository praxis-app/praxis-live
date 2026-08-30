import { api } from '@/client/api-client';
import {
  NotificationContext,
  type NotificationContextValue,
} from '@/components/notifications/notification-context';
import { useAuthData } from '@/hooks/use-auth-data';
import { useServerData } from '@/hooks/use-server-data';
import { useSubscription } from '@/hooks/use-subscription';
import { notificationPubSubTopic } from '@/lib/pub-sub.utils';
import {
  type NotificationPayload,
  type NotificationRes,
  type NotificationsPageRes,
  type UnreadNotificationCountRes,
} from '@/types/notification.types';
import { type PubSubMessage } from '@/types/shared.types';
import {
  type InfiniteData,
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';
import {
  type ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useRef,
} from 'react';

const PAGE_SIZE = 25;

const notificationQueryKey = (userId?: string, serverId?: string) => [
  'notifications',
  userId,
  serverId,
];

const unreadQueryKey = (userId?: string, serverId?: string) => [
  ...notificationQueryKey(userId, serverId),
  'unread-count',
];

const updateNotification = (
  data: InfiniteData<NotificationsPageRes, string | undefined> | undefined,
  notification: NotificationRes,
) => {
  if (!data?.pages[0]) return data;

  const pages = data.pages.map((page) => ({
    ...page,
    notifications: page.notifications.filter(
      (item) => item.id !== notification.id,
    ),
  }));
  pages[0] = {
    ...pages[0],
    notifications: [notification, ...pages[0].notifications],
  };

  return { ...data, pages };
};

const mapNotification = (
  data: InfiniteData<NotificationsPageRes, string | undefined> | undefined,
  notificationId: string,
  transform: (notification: NotificationRes) => NotificationRes,
) => {
  if (!data) return data;
  return {
    ...data,
    pages: data.pages.map((page) => ({
      ...page,
      notifications: page.notifications.map((notification) =>
        notification.id === notificationId
          ? transform(notification)
          : notification,
      ),
    })),
  };
};

interface Props {
  children: ReactNode;
}

export const NotificationProvider = ({ children }: Props) => {
  const { isRegistered, me } = useAuthData();
  const { serverId } = useServerData();
  const queryClient = useQueryClient();
  const previousReadyState = useRef<number | undefined>(undefined);
  const baseTitle = useRef(document.title.replace(/^\(\d+\)\s*/, ''));

  const enabled = isRegistered && !!me && !!serverId;
  const listKey = useMemo(
    () => notificationQueryKey(me?.id, serverId),
    [me?.id, serverId],
  );
  const countKey = useMemo(
    () => unreadQueryKey(me?.id, serverId),
    [me?.id, serverId],
  );

  const notificationsQuery = useInfiniteQuery({
    queryKey: listKey,
    queryFn: ({ pageParam }) => {
      if (!serverId) throw new Error('Current server not found');
      return api.getNotifications(serverId, pageParam, PAGE_SIZE);
    },
    initialPageParam: undefined as string | undefined,
    getNextPageParam: (lastPage) =>
      lastPage.hasMore ? lastPage.nextCursor || undefined : undefined,
    enabled,
  });

  const unreadQuery = useQuery({
    queryKey: countKey,
    queryFn: () => {
      if (!serverId) throw new Error('Current server not found');
      return api.getUnreadNotificationCount(serverId);
    },
    enabled,
  });

  const refresh = useCallback(async () => {
    if (!enabled) return;
    queryClient.setQueryData<
      InfiniteData<NotificationsPageRes, string | undefined>
    >(listKey, (current) =>
      current
        ? {
            ...current,
            pages: current.pages.slice(0, 1),
            pageParams: current.pageParams.slice(0, 1),
          }
        : current,
    );
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: listKey, exact: true }),
      queryClient.invalidateQueries({ queryKey: countKey, exact: true }),
    ]);
  }, [countKey, enabled, listKey, queryClient]);

  const topic = enabled
    ? notificationPubSubTopic(serverId, me?.id)
    : '';
  const { readyState } = useSubscription(topic, {
    enabled,
    onMessage: (event) => {
      const message: PubSubMessage<NotificationPayload & { type: string }> =
        JSON.parse(event.data);
      if (!message.body || message.body.type !== 'notification') return;

      const notification = message.body.notification;
      const current = queryClient.getQueryData<
        InfiniteData<NotificationsPageRes, string | undefined>
      >(listKey);
      const alreadyUnread = current?.pages.some((page) =>
        page.notifications.some(
          (item) => item.id === notification.id && !item.readAt,
        ),
      );
      queryClient.setQueryData(listKey, updateNotification(current, notification));
      if (!alreadyUnread) {
        queryClient.setQueryData<UnreadNotificationCountRes>(
          countKey,
          (value) => ({ unreadCount: (value?.unreadCount || 0) + 1 }),
        );
      }
      void queryClient.invalidateQueries({ queryKey: countKey, exact: true });
    },
  });

  useEffect(() => {
    if (
      previousReadyState.current !== undefined &&
      previousReadyState.current !== WebSocket.OPEN &&
      readyState === WebSocket.OPEN
    ) {
      void refresh();
    }
    previousReadyState.current = readyState;
  }, [readyState, refresh]);

  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible') void refresh();
    };
    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () =>
      document.removeEventListener('visibilitychange', handleVisibilityChange);
  }, [refresh]);

  const unreadCount = unreadQuery.data?.unreadCount || 0;
  useEffect(() => {
    const originalTitle = baseTitle.current;
    document.title = unreadCount
      ? `(${unreadCount}) ${originalTitle}`
      : originalTitle;
    return () => {
      document.title = originalTitle;
    };
  }, [unreadCount]);

  const setReadState = useMutation({
    mutationFn: async ({ notification, read }: { notification: NotificationRes; read: boolean }) => {
      if (!serverId) throw new Error('Current server not found');
      return read
        ? api.markNotificationRead(serverId, notification.id)
        : api.markNotificationUnread(serverId, notification.id);
    },
    onMutate: async ({ notification, read }) => {
      await Promise.all([
        queryClient.cancelQueries({ queryKey: listKey }),
        queryClient.cancelQueries({ queryKey: countKey }),
      ]);
      const previousList = queryClient.getQueryData(listKey);
      const previousCount = queryClient.getQueryData(countKey);
      const wasRead = !!notification.readAt;
      queryClient.setQueryData(listKey, (current: InfiniteData<NotificationsPageRes, string | undefined> | undefined) =>
        mapNotification(current, notification.id, (item) => ({
          ...item,
          readAt: read ? new Date().toISOString() : null,
        })),
      );
      if (wasRead !== read) {
        queryClient.setQueryData<UnreadNotificationCountRes>(countKey, (value) => ({
          unreadCount: Math.max(0, (value?.unreadCount || 0) + (read ? -1 : 1)),
        }));
      }
      return { previousList, previousCount };
    },
    onError: (_error, _variables, context) => {
      queryClient.setQueryData(listKey, context?.previousList);
      queryClient.setQueryData(countKey, context?.previousCount);
    },
    onSuccess: ({ notification }) => {
      queryClient.setQueryData(listKey, (current: InfiniteData<NotificationsPageRes, string | undefined> | undefined) =>
        mapNotification(current, notification.id, () => notification),
      );
    },
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: countKey, exact: true });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: async (notification: NotificationRes) => {
      if (!serverId) throw new Error('Current server not found');
      await api.deleteNotification(serverId, notification.id);
    },
    onMutate: async (notification) => {
      await queryClient.cancelQueries({ queryKey: listKey });
      const previousList = queryClient.getQueryData(listKey);
      const previousCount = queryClient.getQueryData(countKey);
      queryClient.setQueryData(listKey, (current: InfiniteData<NotificationsPageRes, string | undefined> | undefined) =>
        current
          ? {
              ...current,
              pages: current.pages.map((page) => ({
                ...page,
                notifications: page.notifications.filter(
                  (item) => item.id !== notification.id,
                ),
              })),
            }
          : current,
      );
      if (!notification.readAt) {
        queryClient.setQueryData<UnreadNotificationCountRes>(countKey, (value) => ({
          unreadCount: Math.max(0, (value?.unreadCount || 0) - 1),
        }));
      }
      return { previousList, previousCount };
    },
    onError: (_error, _notification, context) => {
      queryClient.setQueryData(listKey, context?.previousList);
      queryClient.setQueryData(countKey, context?.previousCount);
    },
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: countKey, exact: true });
    },
  });

  const markAllMutation = useMutation({
    mutationFn: async () => {
      if (!serverId) throw new Error('Current server not found');
      await api.markAllNotificationsRead(serverId);
    },
    onSuccess: () => void refresh(),
  });
  const clearMutation = useMutation({
    mutationFn: async () => {
      if (!serverId) throw new Error('Current server not found');
      await api.clearNotifications(serverId);
    },
    onSuccess: () => void refresh(),
  });

  const notifications = Array.from(
    new Map(
      notificationsQuery.data?.pages
        .flatMap((page) => page.notifications)
        .map((notification) => [notification.id, notification]),
    ).values(),
  );

  const value: NotificationContextValue = {
    enabled,
    notifications,
    unreadCount,
    isPending: notificationsQuery.isPending,
    isError: notificationsQuery.isError,
    hasNextPage: !!notificationsQuery.hasNextPage,
    isFetchingNextPage: notificationsQuery.isFetchingNextPage,
    fetchNextPage: () => void notificationsQuery.fetchNextPage(),
    refetch: () => void notificationsQuery.refetch(),
    markRead: (notification) =>
      setReadState.mutate({ notification, read: true }),
    markUnread: (notification) =>
      setReadState.mutate({ notification, read: false }),
    deleteNotification: (notification) => deleteMutation.mutate(notification),
    markAllRead: () => markAllMutation.mutate(),
    clearAll: () => clearMutation.mutate(),
  };

  return (
    <NotificationContext.Provider value={value}>
      {children}
    </NotificationContext.Provider>
  );
};
