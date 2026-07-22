import { ServerHomePage } from '@/pages/servers/server-home-page';
import { channelsRouter } from '@/routes/channels.router';
import { serverSettingsRouter } from '@/routes/server-settings.router';
import { type RouteObject } from 'react-router-dom';

export const serversRouter: RouteObject = {
  path: 's/:serverSlug',
  children: [
    {
      index: true,
      element: <ServerHomePage />,
    },
    serverSettingsRouter,
    channelsRouter,
  ],
};
