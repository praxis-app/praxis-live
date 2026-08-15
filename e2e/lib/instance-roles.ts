import { expect, type APIRequestContext } from '@playwright/test';
import {
  authorizationHeaders,
  type AuthenticatedUser,
} from './auth';

type InstanceRole = {
  id: string;
  name: string;
};

type InstanceRolesResponse = {
  instanceRoles: InstanceRole[];
};

export async function grantInstanceAdminRole(
  request: APIRequestContext,
  instanceAdmin: AuthenticatedUser,
  user: AuthenticatedUser,
) {
  const rolesResponse = await request.get('/api/instance/roles', {
    headers: authorizationHeaders(instanceAdmin),
  });

  await expect(rolesResponse).toBeOK();
  const { instanceRoles } = (await rolesResponse.json()) as InstanceRolesResponse;
  const adminRole = instanceRoles.find((role) => role.name === 'admin');
  expect(adminRole).toBeTruthy();

  const membershipResponse = await request.post(
    `/api/instance/roles/${adminRole!.id}/members`,
    {
      headers: authorizationHeaders(instanceAdmin),
      data: { userIds: [user.userId] },
    },
  );

  await expect(membershipResponse).toBeOK();
}
