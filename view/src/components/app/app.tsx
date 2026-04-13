/**
 * TODO: Resolve issue in dev mode when dev server turns off and UI starts to flicker.
 * It's not a big deal, but it's annoying during development sometimes.
 */

import { Outlet } from 'react-router-dom';
import { Layout } from './layout';

export const App = () => (
  <Layout>
    <Outlet />
  </Layout>
);
