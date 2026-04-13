import {
  INSTANCE_PERMISSION_KEYS,
  SERVER_PERMISSION_KEYS,
} from '@/constants/role.constants';
import { t } from 'i18next';
import { type Namespace, type TFunction } from 'react-i18next';
import {
  type InstancePermission,
  type InstancePermissionKeys,
  type ServerPermission,
  type ServerPermissionKeys,
} from '../types/role.types';

export const getServerPermissionValues = (permissions: ServerPermission[]) =>
  SERVER_PERMISSION_KEYS.map((name) => {
    if (name === 'manageChannels') {
      return {
        value: permissions.some(
          (p) => p.subject === 'Channel' && p.action.includes('manage'),
        ),
        name,
      };
    }
    if (name === 'manageServerSettings') {
      return {
        value: permissions.some(
          (p) => p.subject === 'ServerConfig' && p.action.includes('manage'),
        ),
        name,
      };
    }
    if (name === 'manageServerRoles') {
      return {
        value: permissions.some(
          (p) => p.subject === 'ServerRole' && p.action.includes('manage'),
        ),
        name,
      };
    }
    if (name === 'createInvites') {
      return {
        value: permissions.some(
          (p) => p.subject === 'Invite' && p.action.includes('create'),
        ),
        name,
      };
    }
    if (name === 'manageInvites') {
      return {
        value: permissions.some(
          (p) => p.subject === 'Invite' && p.action.includes('manage'),
        ),
        name,
      };
    }
    return {
      value: false,
      name,
    };
  });

export const getInstancePermissionValues = (
  permissions: InstancePermission[],
) =>
  INSTANCE_PERMISSION_KEYS.map((name) => {
    if (name === 'manageInstanceSettings') {
      return {
        value: permissions.some(
          (p) => p.subject === 'InstanceConfig' && p.action.includes('manage'),
        ),
        name,
      };
    }
    if (name === 'manageInstanceRoles') {
      return {
        value: permissions.some(
          (p) => p.subject === 'InstanceRole' && p.action.includes('manage'),
        ),
        name,
      };
    }
    if (name === 'manageServers') {
      return {
        value: permissions.some(
          (p) => p.subject === 'Server' && p.action.includes('manage'),
        ),
        name,
      };
    }
    return {
      value: false,
      name,
    };
  });

export const getServerPermissionValuesMap = (permissions: ServerPermission[]) =>
  getServerPermissionValues(permissions).reduce<Record<string, boolean>>(
    (result, permission) => {
      result[permission.name] = permission.value;
      return result;
    },
    {},
  );

export const getInstancePermissionValuesMap = (
  permissions: InstancePermission[],
) =>
  getInstancePermissionValues(permissions).reduce<Record<string, boolean>>(
    (result, permission) => {
      result[permission.name] = permission.value;
      return result;
    },
    {},
  );

export const getPermissionText = (
  name: ServerPermissionKeys | InstancePermissionKeys,
) => {
  const _t: TFunction<Namespace<'translation'>, undefined> = t;
  return {
    displayName: _t(`permissions.names.${name}`),
    description: _t(`permissions.descriptions.${name}`),
  };
};

type PermissionLike<Subject extends string, Action extends string> = {
  subject: Subject;
  action: Action;
};

type RoleWithPermissions<Subject extends string, Action extends string> = {
  permissions?: PermissionLike<Subject, Action>[];
};

export const buildPermissionRules = <
  Subject extends string,
  Action extends string,
>(
  roles: RoleWithPermissions<Subject, Action>[],
): { subject: Subject; action: Action[] }[] => {
  const permissionMap = roles.reduce<Record<Subject, Action[]>>(
    (result, role) => {
      for (const permission of role.permissions || []) {
        if (!result[permission.subject]) {
          result[permission.subject] = [];
        }
        result[permission.subject].push(permission.action);
      }
      return result;
    },
    {} as Record<Subject, Action[]>,
  );

  return Object.entries(permissionMap).map(([subject, action]) => ({
    subject: subject as Subject,
    action: action as Action[],
  }));
};
