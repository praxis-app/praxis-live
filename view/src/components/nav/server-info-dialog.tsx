import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Separator } from '@/components/ui/separator';
import { INITIAL_SERVER_NAME } from '@/constants/server.constants';
import { type ServerRes } from '@/types/server.types';
import chroma from 'chroma-js';
import ColorHash from 'color-hash';
import { type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

interface Props {
  server?: ServerRes;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  trigger?: ReactNode;
}

export const ServerInfoDialog = ({
  server,
  open,
  onOpenChange,
  trigger,
}: Props) => {
  const { t } = useTranslation();
  const serverName = server?.name || INITIAL_SERVER_NAME;

  const colorHash = new ColorHash();
  const baseColor = colorHash.hex(server?.id || serverName);
  const avatarColors = {
    color: chroma(baseColor).brighten(1.5).hex(),
    backgroundColor: chroma(baseColor).darken(1.35).hex(),
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
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
              <div className="truncate text-xl font-semibold">{serverName}</div>
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
      </DialogContent>
    </Dialog>
  );
};
