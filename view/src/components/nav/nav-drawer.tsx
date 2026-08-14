import {
  CreateChannelForm,
  CreateChannelFormSubmitButton,
} from '@/components/channels/create-channel-form';
import { ServerInfoDialog } from '@/components/nav/server-info-dialog';
import { SwitchServerDialog } from '@/components/nav/switch-server-dialog';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import {
  Drawer,
  DrawerContent,
  DrawerDescription,
  DrawerHeader,
  DrawerTitle,
  DrawerTrigger,
} from '@/components/ui/drawer';
import { Separator } from '@/components/ui/separator';
import { INITIAL_SERVER_NAME } from '@/constants/server.constants';
import { NavigationPaths } from '@/constants/shared.constants';
import { useAbility } from '@/hooks/use-ability';
import { useMeQuery } from '@/hooks/use-me-query';
import { useServerData } from '@/hooks/use-server-data';
import { useNavStore } from '@/store/nav.store';
import { VisuallyHidden } from '@radix-ui/react-visually-hidden';
import { type ReactNode, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { MdAddCircle, MdOutlineSettings, MdSettings } from 'react-icons/md';
import { TbSwitchHorizontal } from 'react-icons/tb';
import { useNavigate } from 'react-router-dom';

interface Props {
  trigger: ReactNode;
}

export const NavDrawer = ({ trigger }: Props) => {
  const { setIsNavSheetOpen } = useNavStore();
  const [showNavDrawer, setShowNavDrawer] = useState(false);
  const [showRoomFormDialog, setShowRoomFormDialog] = useState(false);
  const [showServerSwitchDialog, setShowServerSwitchDialog] = useState(false);

  const { t } = useTranslation();
  const navigate = useNavigate();

  const { serverAbility, instanceAbility } = useAbility();
  const { server, serverPath } = useServerData();
  const { data: meData } = useMeQuery();
  const serverName = server?.name || INITIAL_SERVER_NAME;
  const canManageChannels = serverAbility.can('manage', 'Channel');
  const canManageServerSettings = serverAbility.can('manage', 'ServerConfig');
  const canManageInstanceSettings = instanceAbility.can(
    'manage',
    'InstanceConfig',
  );
  const hasMultipleServers = !!meData && meData.user.serversCount > 1;
  const hasServerMenuActions =
    canManageChannels ||
    canManageServerSettings ||
    canManageInstanceSettings ||
    hasMultipleServers;

  return (
    <>
      <Drawer open={showNavDrawer} onOpenChange={setShowNavDrawer}>
        <DrawerTrigger asChild>{trigger}</DrawerTrigger>

        <DrawerContent className="flex min-h-[calc(100%-68px)] flex-col items-start rounded-t-2xl border-0">
          <VisuallyHidden>
            <DrawerHeader>
              <DrawerTitle>{t('navigation.titles.navDrawer')}</DrawerTitle>
              <DrawerDescription>
                {t('navigation.descriptions.navDrawer')}
              </DrawerDescription>
            </DrawerHeader>
          </VisuallyHidden>

          <div className="flex w-full flex-col items-start gap-4 p-5">
            <ServerInfoDialog
              server={server}
              trigger={
                <Button
                  variant="ghost"
                  className="text-md w-full justify-start font-medium"
                >
                  {serverName}
                </Button>
              }
            />

            {hasServerMenuActions && <Separator />}

            {canManageChannels && (
              <Dialog
                open={showRoomFormDialog}
                onOpenChange={setShowRoomFormDialog}
              >
                <DialogTrigger asChild>
                  <Button
                    variant="ghost"
                    className="text-md flex items-center gap-6 font-normal"
                  >
                    <MdAddCircle className="size-6" />
                    {t('channels.actions.create')}
                  </Button>
                </DialogTrigger>
                <DialogContent>
                  <DialogHeader>
                    <DialogTitle>
                      {t('channels.prompts.createChannel')}
                    </DialogTitle>
                    <DialogDescription>
                      {t('channels.prompts.startConversation')}
                    </DialogDescription>
                  </DialogHeader>

                  <CreateChannelForm
                    submitButton={(props) => (
                      <DialogFooter>
                        <CreateChannelFormSubmitButton {...props} />
                      </DialogFooter>
                    )}
                    onSubmit={() => {
                      setShowNavDrawer(false);
                      setShowRoomFormDialog(false);
                      setIsNavSheetOpen(false);
                    }}
                  />
                </DialogContent>
              </Dialog>
            )}

            {canManageInstanceSettings && (
              <Button
                variant="ghost"
                className="text-md flex items-center gap-6 font-normal"
                onClick={() => {
                  navigate(NavigationPaths.Settings);
                  setIsNavSheetOpen(false);
                }}
              >
                <MdOutlineSettings className="size-6" />
                {t('navigation.labels.instanceSettings')}
              </Button>
            )}

            {canManageServerSettings && (
              <Button
                variant="ghost"
                className="text-md flex items-center gap-6 font-normal"
                onClick={() => {
                  navigate(`${serverPath}${NavigationPaths.Settings}`);
                  setIsNavSheetOpen(false);
                }}
              >
                <MdSettings className="size-6" />
                {t('navigation.labels.serverSettings')}
              </Button>
            )}

            {hasMultipleServers && (
              <Button
                variant="ghost"
                className="text-md flex items-center gap-6 font-normal"
                onClick={() => setShowServerSwitchDialog(true)}
              >
                <TbSwitchHorizontal className="size-6" />
                {t('navigation.labels.switchServers')}
              </Button>
            )}
          </div>
        </DrawerContent>
      </Drawer>

      <SwitchServerDialog
        open={showServerSwitchDialog}
        onOpenChange={setShowServerSwitchDialog}
        onSelect={() => setIsNavSheetOpen(false)}
      />
    </>
  );
};
