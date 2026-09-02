import { api } from '@/client/api-client';
import { TopNav } from '@/components/nav/top-nav';
import { NotificationSettingsForm } from '@/components/settings/notification-settings-form';
import { Card, CardContent } from '@/components/ui/card';
import { Container } from '@/components/ui/container';
import { UserProfileForm } from '@/components/users/user-profile-form';
import { NavigationPaths } from '@/constants/shared.constants';
import { useMeQuery } from '@/hooks/use-me-query';
import { useQuery } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { MdClose } from 'react-icons/md';
import { Navigate } from 'react-router-dom';

export const UserSettings = () => {
  const { t } = useTranslation();

  const { data: meData } = useMeQuery();
  const userId = meData?.user?.id || '';

  const { data: profileData } = useQuery({
    queryKey: ['users', userId, 'profile'],
    queryFn: () => api.getUserProfile(userId),
    enabled: !!userId,
  });

  const {
    data: userConfigData,
    isPending: isUserConfigPending,
    error: userConfigError,
  } = useQuery({
    queryKey: ['users', 'me', 'configs'],
    queryFn: () => api.getUserConfig(),
    enabled: !!userId,
  });

  if (!meData?.user) {
    return null;
  }

  if (meData.user.anonymous) {
    return <Navigate to={NavigationPaths.Explore} />;
  }

  return (
    <>
      <TopNav
        header={t('settings.headers.userSettings')}
        backBtnIcon={<MdClose className="size-6" />}
        goBackOnEscape
      />

      <Container className="flex flex-col gap-8">
        <section aria-labelledby="user-profile-settings-heading">
          <h2
            id="user-profile-settings-heading"
            className="text-lg font-semibold"
          >
            {t('navigation.labels.profile')}
          </h2>
          <p className="text-muted-foreground mt-1 mb-4 text-sm">
            {t('settings.descriptions.profileSection')}
          </p>
          <Card>
            <CardContent>
              {profileData?.user && (
                <UserProfileForm
                  userProfile={profileData.user}
                  me={meData.user}
                />
              )}
            </CardContent>
          </Card>
        </section>

        <section aria-labelledby="user-notification-settings-heading">
          <h2
            id="user-notification-settings-heading"
            className="text-lg font-semibold"
          >
            {t('navigation.labels.notifications')}
          </h2>
          <p className="text-muted-foreground mt-1 mb-4 text-sm">
            {t('settings.descriptions.notificationSection')}
          </p>
          <Card>
            <CardContent>
              {userConfigError && <p>{t('errors.somethingWentWrong')}</p>}
              {!userConfigError && !isUserConfigPending && (
                <NotificationSettingsForm
                  userConfig={userConfigData.userConfig}
                />
              )}
            </CardContent>
          </Card>
        </section>
      </Container>
    </>
  );
};
