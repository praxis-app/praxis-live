import { type RouteObject } from 'react-router-dom';
import { EditUserProfile } from '../pages/users/edit-user-profile';
import { UserSettings } from '../pages/users/user-settings';

export const usersRouter: RouteObject = {
  path: '/users',
  children: [
    {
      path: 'edit',
      element: <EditUserProfile />,
    },
    {
      path: 'settings',
      element: <UserSettings />,
    },
  ],
};
