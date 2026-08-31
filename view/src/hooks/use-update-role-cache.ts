import {
  getMergedInstancePermissions,
  getMergedServerPermissions,
} from '@/lib/role.utils';
import { type InstanceRoleRes, type ServerRoleRes } from '@/types/role.types';
import { type CurrentUser } from '@/types/user.types';
import { useQueryClient } from '@tanstack/react-query';

/**
 * Keeps the cached roles list and the abilities held by `me` in step after a
 * role changes. The cached permissions are the union of every role the user
 * belongs to, so they are recomputed from the updated list rather than
 * overwritten with a single role.
 *
 * Returns false when the roles list isn't cached and the union can't be
 * derived, leaving the caller to invalidate instead of guessing.
 */
export const useUpdateRoleCache = () => {
  const queryClient = useQueryClient();

  const updateCachedServerRoles = (
    serverId: string | undefined,
    updateRoles: (serverRoles: ServerRoleRes[]) => ServerRoleRes[],
  ) => {
    const meData = queryClient.getQueryData<{ user: CurrentUser }>(['me']);
    const rolesData = queryClient.getQueryData<{
      serverRoles: ServerRoleRes[];
    }>(['servers', serverId, 'roles']);

    if (!serverId || !meData || !rolesData) {
      return false;
    }
    const serverRoles = updateRoles(rolesData.serverRoles);

    queryClient.setQueryData(['servers', serverId, 'roles'], { serverRoles });
    queryClient.setQueryData<{ user: CurrentUser }>(['me'], {
      user: {
        ...meData.user,
        permissions: {
          ...meData.user.permissions,
          servers: {
            ...meData.user.permissions.servers,
            [serverId]: getMergedServerPermissions(serverRoles, meData.user.id),
          },
        },
      },
    });
    return true;
  };

  const updateCachedInstanceRoles = (
    updateRoles: (instanceRoles: InstanceRoleRes[]) => InstanceRoleRes[],
  ) => {
    const meData = queryClient.getQueryData<{ user: CurrentUser }>(['me']);
    const rolesData = queryClient.getQueryData<{
      instanceRoles: InstanceRoleRes[];
    }>(['instance-roles']);

    if (!meData || !rolesData) {
      return false;
    }
    const instanceRoles = updateRoles(rolesData.instanceRoles);

    queryClient.setQueryData(['instance-roles'], { instanceRoles });
    queryClient.setQueryData<{ user: CurrentUser }>(['me'], {
      user: {
        ...meData.user,
        permissions: {
          ...meData.user.permissions,
          instance: getMergedInstancePermissions(instanceRoles, meData.user.id),
        },
      },
    });
    return true;
  };

  return { updateCachedServerRoles, updateCachedInstanceRoles };
};
