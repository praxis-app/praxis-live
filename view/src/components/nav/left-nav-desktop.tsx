import appIconImg from '@/assets/images/app-icon.png';
import { ChannelListDesktop } from '@/components/channels/channel-list-desktop';
import {
  CreateChannelForm,
  CreateChannelFormSubmitButton,
} from '@/components/channels/create-channel-form';
import { CurrentServerMenuLabel } from '@/components/nav/current-server-menu-label';
import { LeftNavUserMenu } from '@/components/nav/left-nav-user-menu';
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
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { NavigationPaths } from '@/constants/shared.constants';
import { useAbility } from '@/hooks/use-ability';
import { useAuthData } from '@/hooks/use-auth-data';
import { useServerData } from '@/hooks/use-server-data';
import { cn } from '@/lib/shared.utils';
import { useAppStore } from '@/store/app.store';
import { useAuthStore } from '@/store/auth.store';
import { type CurrentUserRes } from '@/types/user.types';
import { INITIAL_SERVER_NAME } from '@/constants/server.constants';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  MdAddCircle,
  MdExpandMore,
  MdEvent,
  MdOutlineSettings,
  MdRocketLaunch,
  MdSettings,
} from 'react-icons/md';
import { TbSwitchHorizontal } from 'react-icons/tb';
import { Link, useLocation } from 'react-router-dom';
import { toast } from 'sonner';

interface Props {
  me?: CurrentUserRes;
}

export const LeftNavDesktop = ({ me }: Props) => {
  const { isLoggedIn } = useAuthStore();
  const { isAppLoading } = useAppStore();

  const [showRoomFormDialog, setShowRoomFormDialog] = useState(false);
  const [showServerInfoDialog, setShowServerInfoDialog] = useState(false);
  const [showServerSwitchDialog, setShowServerSwitchDialog] = useState(false);

  const { t } = useTranslation();
  const location = useLocation();
  const { server, serverPath, myServerCount } = useServerData();
  const { signUpPath } = useAuthData();

  const { serverAbility, instanceAbility } = useAbility();
  const canManageChannels = serverAbility.can('manage', 'Channel');
  const canManageServerSettings = serverAbility.can('manage', 'ServerConfig');
  const canManageInstanceSettings = instanceAbility.can(
    'manage',
    'InstanceConfig',
  );
  const hasMultipleServers = !!myServerCount && myServerCount > 1;
  const hasServerMenuActions =
    canManageServerSettings ||
    canManageChannels ||
    canManageInstanceSettings ||
    hasMultipleServers;

  const serverName = server?.name || INITIAL_SERVER_NAME;
  const eventsActive = location.pathname.startsWith(`${serverPath}/events`);

  return (
    <div className="dark:bg-card bg-secondary flex h-full w-60 flex-col border-r border-[--color-border]">
      <SwitchServerDialog
        open={showServerSwitchDialog}
        onOpenChange={setShowServerSwitchDialog}
      />
      <Dialog open={showRoomFormDialog} onOpenChange={setShowRoomFormDialog}>
        <DropdownMenu>
          <DropdownMenuTrigger className="hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50 flex h-13.75 w-full cursor-pointer justify-between border-b border-[--color-border] pr-3 pl-4 select-none focus:outline-none">
            <div className="flex min-w-0 items-center gap-2">
              <img
                src={appIconImg}
                alt={INITIAL_SERVER_NAME}
                className="size-[1.55rem] self-center"
              />
              <div className="self-center truncate text-base/tight font-medium tracking-[0.02em]">
                {INITIAL_SERVER_NAME}
              </div>
            </div>

            <MdExpandMore className="size-[1.4rem] shrink-0 self-center" />
          </DropdownMenuTrigger>
          <DropdownMenuContent sideOffset={10} className="w-52">
            <DropdownMenuItem
              className="text-md items-start py-2.5"
              onSelect={() => setShowServerInfoDialog(true)}
            >
              <CurrentServerMenuLabel serverName={serverName} />
            </DropdownMenuItem>
            <DropdownMenuSeparator />

            {canManageChannels && (
              <DialogTrigger asChild>
                <DropdownMenuItem className="text-md">
                  <MdAddCircle className="text-foreground size-5" />
                  {t('channels.actions.create')}
                </DropdownMenuItem>
              </DialogTrigger>
            )}

            {canManageInstanceSettings && (
              <Link to={NavigationPaths.Settings}>
                <DropdownMenuItem className="text-md">
                  <MdOutlineSettings className="text-foreground size-5" />
                  {t('navigation.labels.instanceSettings')}
                </DropdownMenuItem>
              </Link>
            )}

            {canManageServerSettings && (
              <Link to={`${serverPath}${NavigationPaths.Settings}`}>
                <DropdownMenuItem className="text-md">
                  <MdSettings className="text-foreground size-5" />
                  {t('navigation.labels.serverSettings')}
                </DropdownMenuItem>
              </Link>
            )}

            {me && me.serversCount > 1 && (
              <DropdownMenuItem
                className="text-md"
                onSelect={() => setShowServerSwitchDialog(true)}
              >
                <TbSwitchHorizontal className="text-foreground size-5" />
                {t('navigation.labels.switchServers')}
              </DropdownMenuItem>
            )}

            {hasServerMenuActions && <DropdownMenuSeparator />}

            <Link to={NavigationPaths.About}>
              <DropdownMenuItem className="text-md">
                <MdRocketLaunch className="text-foreground size-5" />
                {t('landing.actions.backToLanding')}
              </DropdownMenuItem>
            </Link>
          </DropdownMenuContent>
        </DropdownMenu>

        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('channels.prompts.createChannel')}</DialogTitle>
          </DialogHeader>
          <DialogDescription>
            {t('channels.prompts.startConversation')}
          </DialogDescription>

          <CreateChannelForm
            submitButton={(props) => (
              <DialogFooter>
                <CreateChannelFormSubmitButton {...props} />
              </DialogFooter>
            )}
            onSubmit={() => setShowRoomFormDialog(false)}
            className="min-w-100"
          />
        </DialogContent>
      </Dialog>

      <ServerInfoDialog
        server={server}
        open={showServerInfoDialog}
        onOpenChange={setShowServerInfoDialog}
        canSwitchServers={hasMultipleServers}
      />

      <div className="border-b border-[--color-border] p-2">
        <Link
          to={`${serverPath}${NavigationPaths.Events}`}
          className={cn(
            'text-muted-foreground hover:bg-foreground/10 active:bg-foreground/15 dark:hover:bg-accent dark:active:bg-accent/80 flex items-center gap-2 rounded-lg px-2 py-[0.225rem] text-[0.925rem]',
            eventsActive && 'bg-foreground/10 text-foreground dark:bg-accent',
          )}
          aria-current={eventsActive ? 'page' : undefined}
        >
          <MdEvent className="size-6" />
          {t('navigation.labels.events')}
        </Link>
      </div>

      <ChannelListDesktop />

      <div className="flex h-15 items-center justify-between border-t border-[--color-border] px-1.5">
        <LeftNavUserMenu />

        {isLoggedIn ? (
          <Button
            onClick={() => toast(t('prompts.inDev'))}
            variant="ghost"
            size="icon"
          >
            <MdSettings className="text-muted-foreground size-6" />
          </Button>
        ) : (
          <div
            className={cn(
              'flex w-full justify-center gap-2',
              isAppLoading && 'hidden',
            )}
          >
            <Link to={NavigationPaths.Login}>
              <Button variant="ghost">{t('auth.actions.logIn')}</Button>
            </Link>
            <Link to={signUpPath}>
              <Button variant="ghost">{t('auth.actions.signUp')}</Button>
            </Link>
          </div>
        )}
      </div>
    </div>
  );
};
