import { describe, expect, it } from 'vitest';
import {
  getMergedInstancePermissions,
  getMergedServerPermissions,
} from '../role.utils';
import {
  type InstanceRoleRes,
  type InstancePermission,
  type ServerPermission,
  type ServerRoleRes,
} from '@/types/role.types';
import { type UserRes } from '@/types/user.types';

const ME = 'me-id';
const OTHER = 'other-id';

const user = (id: string) => ({ id }) as UserRes;

const serverRole = (
  id: string,
  memberIds: string[],
  permissions: ServerPermission[],
): ServerRoleRes => ({
  id,
  name: id,
  color: '#000000',
  permissions,
  memberCount: memberIds.length,
  members: memberIds.map(user),
});

const instanceRole = (
  id: string,
  memberIds: string[],
  permissions: InstancePermission[],
): InstanceRoleRes => ({
  id,
  name: id,
  color: '#000000',
  permissions,
  memberCount: memberIds.length,
  members: memberIds.map(user),
});

describe('getMergedServerPermissions', () => {
  it('ignores roles the user does not belong to', () => {
    const roles = [
      serverRole(
        'a',
        [OTHER],
        [{ subject: 'ProposalBlock', action: ['create'] }],
      ),
    ];
    expect(getMergedServerPermissions(roles, ME)).toEqual([]);
  });

  it('unions permissions across every role the user belongs to', () => {
    const roles = [
      serverRole('a', [ME], [{ subject: 'ProposalBlock', action: ['create'] }]),
      serverRole('b', [ME], [{ subject: 'Channel', action: ['manage'] }]),
      serverRole('c', [OTHER], [{ subject: 'Invite', action: ['manage'] }]),
    ];
    expect(getMergedServerPermissions(roles, ME)).toEqual([
      { subject: 'Channel', action: ['manage'] },
      { subject: 'ProposalBlock', action: ['create'] },
    ]);
  });

  it('merges actions for a subject granted by more than one role', () => {
    const roles = [
      serverRole('a', [ME], [{ subject: 'Invite', action: ['create'] }]),
      serverRole('b', [ME], [{ subject: 'Invite', action: ['manage'] }]),
    ];
    expect(getMergedServerPermissions(roles, ME)).toEqual([
      { subject: 'Invite', action: ['create', 'manage'] },
    ]);
  });

  it('does not duplicate an action granted by two roles', () => {
    const roles = [
      serverRole('a', [ME], [{ subject: 'Invite', action: ['manage'] }]),
      serverRole('b', [ME], [{ subject: 'Invite', action: ['manage'] }]),
    ];
    expect(getMergedServerPermissions(roles, ME)).toEqual([
      { subject: 'Invite', action: ['manage'] },
    ]);
  });

  it('keeps a permission the user still holds through another role', () => {
    const roles = [
      // Blocking was just switched off here, but role `b` still grants it.
      serverRole('a', [ME], []),
      serverRole('b', [ME], [{ subject: 'ProposalBlock', action: ['create'] }]),
    ];
    expect(getMergedServerPermissions(roles, ME)).toEqual([
      { subject: 'ProposalBlock', action: ['create'] },
    ]);
  });
});

describe('getMergedInstancePermissions', () => {
  it('unions permissions across the user’s instance roles', () => {
    const roles = [
      instanceRole('a', [ME], [{ subject: 'Server', action: ['manage'] }]),
      instanceRole(
        'b',
        [ME],
        [{ subject: 'InstanceRole', action: ['manage'] }],
      ),
      instanceRole(
        'c',
        [OTHER],
        [{ subject: 'InstanceConfig', action: ['manage'] }],
      ),
    ];
    expect(getMergedInstancePermissions(roles, ME)).toEqual([
      { subject: 'InstanceRole', action: ['manage'] },
      { subject: 'Server', action: ['manage'] },
    ]);
  });
});
