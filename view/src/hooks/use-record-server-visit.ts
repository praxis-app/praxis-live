import { api } from '@/client/api-client';
import { useAuthData } from '@/hooks/use-auth-data';
import { useServerData } from '@/hooks/use-server-data';
import { type CurrentUser } from '@/types/user.types';
import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';

// Records the visit and keeps the cached `me.currentServer` in sync whenever
// the resolved server changes. `me` is long lived (30 minute stale time, no
// refetch on mount) and is the only source of the current server on slug-less
// routes such as `/`, so without this it keeps pointing at whichever server
// the session started on.
//
// Mount this once, from `AuthWrapper`. It must not live in `useServerData`:
// that hook is a read used by dozens of components, so a write there fires one
// request per mounted consumer instead of one per visit.
export const useRecordServerVisit = () => {
  const queryClient = useQueryClient();

  const { server } = useServerData();
  const { isMeSuccess } = useAuthData();

  const recordedServerId = useRef<string | null>(null);

  useEffect(() => {
    if (!server || !isMeSuccess || recordedServerId.current === server.id) {
      return;
    }
    recordedServerId.current = server.id;

    api.setCurrentServer(server.id).catch(() => {
      // Best-effort: a failed write here should not block viewing the server.
    });
    queryClient.setQueryData<{ user: CurrentUser }>(['me'], (oldData) => {
      if (!oldData || oldData.user.currentServer?.id === server.id) {
        return oldData;
      }
      return { user: { ...oldData.user, currentServer: server } };
    });
  }, [queryClient, server, isMeSuccess]);
};
