import { ChannelSkeleton } from '@/components/channels/channel-skeleton';
import { useAuthData } from '@/hooks/use-auth-data';
import { useAppStore } from '@/store/app.store';
import { PublicProductLandingPage } from './landing/public-product-landing-page';
import { RegisteredHomePage } from './landing/registered-home-page';

export const HomePage = () => {
  const { isLoggedIn, isMeLoading, isRegistered } = useAuthData();
  const { isAppLoading } = useAppStore();

  if (isAppLoading || (isLoggedIn && isMeLoading)) {
    return <ChannelSkeleton />;
  }

  if (isRegistered) {
    return <RegisteredHomePage />;
  }

  return <PublicProductLandingPage />;
};
