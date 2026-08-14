import { SwitchServerDialog } from '@/components/nav/switch-server-dialog';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
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
import { Separator } from '@/components/ui/separator';
import { INITIAL_SERVER_NAME } from '@/constants/server.constants';
import { type ServerRes } from '@/types/server.types';
import chroma from 'chroma-js';
import ColorHash from 'color-hash';
import { type ReactNode, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { TbSwitchHorizontal } from 'react-icons/tb';

interface Props {
  server?: ServerRes;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  canSwitchServers?: boolean;
  onServerSelect?(): void;
  trigger?: ReactNode;
}

export const ServerInfoDialog = ({
  server,
  open,
  onOpenChange,
  canSwitchServers = false,
  onServerSelect,
  trigger,
}: Props) => {
  const [internalOpen, setInternalOpen] = useState(false);
  const [showServerSwitchDialog, setShowServerSwitchDialog] = useState(false);

  const { t } = useTranslation();
  const serverName = server?.name || INITIAL_SERVER_NAME;
  const isOpen = open ?? internalOpen;

  const handleOpenChange = (nextOpen: boolean) => {
    if (open === undefined) {
      setInternalOpen(nextOpen);
    }
    onOpenChange?.(nextOpen);
  };

  const handleSwitchServers = () => {
    handleOpenChange(false);
    setShowServerSwitchDialog(true);
  };

  const colorHash = new ColorHash();
  const baseColor = colorHash.hex(server?.id || serverName);
  const avatarColors = {
    color: chroma(baseColor).brighten(1.5).hex(),
    backgroundColor: chroma(baseColor).darken(1.35).hex(),
  };

  return (
    <>
      <Dialog open={isOpen} onOpenChange={handleOpenChange}>
        {trigger && <DialogTrigger asChild>{trigger}</DialogTrigger>}
        <DialogContent className="md:w-xl">
          <DialogHeader className="pr-10 text-left">
            <DialogTitle>{t('servers.headers.details')}</DialogTitle>
            <DialogDescription>
              {t('servers.descriptions.currentServer')}
            </DialogDescription>
          </DialogHeader>

          <Separator />

          <div className="flex min-h-44 flex-col gap-6 py-2">
            <div className="flex min-w-0 items-center gap-4">
              <Avatar className="size-12 shrink-0">
                <AvatarFallback
                  className="text-xl font-light uppercase"
                  style={avatarColors}
                >
                  {serverName.trim()[0] || '?'}
                </AvatarFallback>
              </Avatar>
              <div className="min-w-0">
                <div className="text-muted-foreground text-xs font-medium tracking-wide uppercase">
                  {t('servers.labels.current')}
                </div>
                <div className="truncate text-xl font-semibold">
                  {serverName}
                </div>
              </div>
            </div>

            <Separator />

            <div>
              <div className="text-sm font-medium">
                {t('servers.form.description')}
              </div>
              <p className="text-muted-foreground mt-2 text-base leading-relaxed">
                {server?.description ||
                  t('servers.descriptions.noDescription')}
              </p>
            </div>
          </div>

          {canSwitchServers && (
            <DialogFooter className="mt-auto md:mt-0">
              <Button
                variant="outline"
                className="w-full md:w-auto"
                onClick={handleSwitchServers}
              >
                <TbSwitchHorizontal />
                {t('navigation.labels.switchServers')}
              </Button>
            </DialogFooter>
          )}
        </DialogContent>
      </Dialog>

      <SwitchServerDialog
        open={showServerSwitchDialog}
        onOpenChange={setShowServerSwitchDialog}
        onSelect={onServerSelect}
      />
    </>
  );
};
