import { TopNav } from '@/components/nav/top-nav';
import { PermissionDenied } from '@/components/shared/permission-denied';
import { SettingsNavItem } from '@/components/settings/settings-nav-item';
import { Container } from '@/components/ui/container';
import { NavigationPaths } from '@/constants/shared.constants';
import { useAbility } from '@/hooks/use-ability';
import { useServerData } from '@/hooks/use-server-data';
import { useTranslation } from 'react-i18next';
import {
  MdAdminPanelSettings,
  MdClose,
  MdEmojiPeople,
  MdGroups,
  MdLink,
  MdSettings,
} from 'react-icons/md';

export const Settings = () => {
  const { t } = useTranslation();
  const { serverPath } = useServerData();
  const { serverAbility, instanceAbility, isLoading } = useAbility();

  const canManageServerSettings = serverAbility.can('manage', 'ServerConfig');
  const canManageInstanceSettings = instanceAbility.can(
    'manage',
    'InstanceConfig',
  );

  if (isLoading) {
    return null;
  }

  if (!canManageServerSettings && !canManageInstanceSettings) {
    return (
      <PermissionDenied
        topNavProps={{ header: t('navigation.headers.settings') }}
      />
    );
  }

  return (
    <>
      <TopNav
        header={t('navigation.headers.settings')}
        backBtnIcon={<MdClose className="size-6" />}
        goBackOnEscape
      />

      <Container className="flex flex-col gap-8">
        {canManageServerSettings && (
          <section aria-labelledby="server-settings-heading">
            <h2 id="server-settings-heading" className="text-lg font-semibold">
              {t('navigation.labels.serverSettings')}
            </h2>
            <p className="text-muted-foreground mt-1 mb-4 text-sm">
              {t('settings.descriptions.serverSection')}
            </p>
            <div className="flex flex-col gap-3">
              <SettingsNavItem
                Icon={MdSettings}
                label={t('navigation.labels.general')}
                to={`${serverPath}${NavigationPaths.GeneralSettings}`}
              />
              <SettingsNavItem
                Icon={MdEmojiPeople}
                label={t('navigation.labels.proposals')}
                to={`${serverPath}${NavigationPaths.ProposalSettings}`}
              />
              <SettingsNavItem
                Icon={MdAdminPanelSettings}
                label={t('navigation.labels.serverRoles')}
                to={`${serverPath}${NavigationPaths.Roles}`}
              />
              <SettingsNavItem
                Icon={MdLink}
                label={t('navigation.labels.invites')}
                to={`${serverPath}${NavigationPaths.Invites}`}
              />
            </div>
          </section>
        )}

        {canManageInstanceSettings && (
          <section aria-labelledby="instance-settings-heading">
            <h2
              id="instance-settings-heading"
              className="text-lg font-semibold"
            >
              {t('navigation.labels.instanceSettings')}
            </h2>
            <p className="text-muted-foreground mt-1 mb-4 text-sm">
              {t('settings.descriptions.instanceSection')}
            </p>
            <div className="flex flex-col gap-3">
              <SettingsNavItem
                Icon={MdAdminPanelSettings}
                label={t('navigation.labels.instanceRoles')}
                to={NavigationPaths.Roles}
              />
              <SettingsNavItem
                Icon={MdGroups}
                label={t('settings.headers.manageServers')}
                to={NavigationPaths.ManageServers}
              />
            </div>
          </section>
        )}
      </Container>
    </>
  );
};
