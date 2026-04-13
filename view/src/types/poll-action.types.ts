import {
  POLL_ACTION_TYPE,
  ROLE_ATTRIBUTE_CHANGE_TYPE,
} from '@/constants/poll-action.constants';
import { type AbilityAction, type ServerAbilitySubject } from '@/types/role.types';
import { type UserRes } from './user.types';

export type PollActionType = (typeof POLL_ACTION_TYPE)[number];

export type RoleAttributeChangeType =
  (typeof ROLE_ATTRIBUTE_CHANGE_TYPE)[number];

// -------------------------------------------------------------------------
// Requests
// -------------------------------------------------------------------------

export interface PollActionReq {
  actionType: PollActionType;
  serverRole?: PollActionServerRoleReq;
}

export interface PollActionServerRoleReq {
  name?: string;
  color?: string;
  prevName?: string;
  prevColor?: string;
  members?: PollActionServerRoleMemberReq[];
  permissions?: PollActionServerRolePermissionReq[];
}

export interface PollActionServerRoleMemberReq {
  userId: string;
  changeType: RoleAttributeChangeType;
}

export interface PollActionServerRolePermissionReq {
  subject: ServerAbilitySubject;
  action: AbilityAction;
  changeType: RoleAttributeChangeType;
}

export interface CreatePollActionReq {
  actionType: PollActionType;
  serverRole?: CreatePollActionServerRoleReq;
}

export interface CreatePollActionServerRoleReq {
  name?: string;
  color?: string;
  members?: CreatePollActionServerRoleMemberReq[];
  permissions?: CreatePollActionServerRolePermissionReq[];
  serverRoleToUpdateId?: string;
}

export interface CreatePollActionServerRoleMemberReq {
  userId: string;
  changeType: RoleAttributeChangeType;
}

export interface CreatePollActionServerRolePermissionReq {
  subject: ServerAbilitySubject;
  actions: {
    action: AbilityAction;
    changeType: RoleAttributeChangeType;
  }[];
}

// -------------------------------------------------------------------------
// Responses
// -------------------------------------------------------------------------

export interface PollActionRes {
  id: string;
  actionType: PollActionType;
  serverRole?: PollActionServerRoleRes;
}

export interface PollActionServerRoleRes {
  id: string;
  name?: string;
  color?: string;
  prevName?: string;
  prevColor?: string;
  serverRoleId: string;
  members?: PollActionServerRoleMemberRes[];
  permissions?: PollActionServerRolePermissionRes[];
}

export interface PollActionServerRoleMemberRes {
  changeType: RoleAttributeChangeType;
  user: UserRes;
}

export interface PollActionServerRolePermissionRes {
  subject: ServerAbilitySubject;
  action: AbilityAction;
  changeType: RoleAttributeChangeType;
}
