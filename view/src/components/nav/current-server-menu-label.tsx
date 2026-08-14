import { useTranslation } from 'react-i18next';
import { LuServer } from 'react-icons/lu';

interface Props {
  serverName: string;
}

export const CurrentServerMenuLabel = ({ serverName }: Props) => {
  const { t } = useTranslation();

  return (
    <div className="flex w-full min-w-0 flex-col items-start gap-1 text-left">
      <span className="text-muted-foreground text-xs font-normal">
        {t('servers.labels.current')}
      </span>
      <div className="flex min-w-0 items-center gap-2">
        <LuServer className="text-muted-foreground size-5 shrink-0" />
        <span className="min-w-0 truncate font-medium">{serverName}</span>
      </div>
    </div>
  );
};
