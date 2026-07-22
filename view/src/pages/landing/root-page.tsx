import { ChannelSkeleton } from '@/components/channels/channel-skeleton';
import { useAuthData } from '@/hooks/use-auth-data';
import { ServerHomePage } from '@/pages/servers/server-home-page';
import { useAppStore } from '@/store/app.store';
import { PublicProductLandingPage } from './public-product-landing-page';

export const RootPage = () => {
  const { isLoggedIn, isMeLoading, isRegistered } = useAuthData();
  const { isAppLoading } = useAppStore();

  if (isAppLoading || (isLoggedIn && isMeLoading)) {
    return <ChannelSkeleton />;
  }

  if (isRegistered) {
    return <ServerHomePage />;
  }

  return <PublicProductLandingPage />;
};
