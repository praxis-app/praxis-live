import { type PollActionServerConfigRes } from '@/types/poll-action.types';
import { useTranslation } from 'react-i18next';
import { SERVER_CONFIG_FIELDS } from './server-config-changes.utils';

const fields = SERVER_CONFIG_FIELDS;

const formatValue = (value: unknown, t: (key: string) => string) =>
  typeof value === 'boolean'
    ? t(value ? 'actions.enabled' : 'actions.disabled')
    : String(value);

export const ServerConfigChanges = ({
  changes,
}: {
  changes: PollActionServerConfigRes;
}) => {
  const { t } = useTranslation();
  return (
    <div className="space-y-3">
      {fields.map((field) => {
        const value = changes[field];
        const previous = changes[`prev${field[0].toUpperCase()}${field.slice(1)}` as keyof PollActionServerConfigRes];
        if (value === undefined || previous === undefined) return null;
        return (
          <div key={field} className="space-y-1">
            <div className="text-sm font-medium">{t(`settings.names.${field}`)}</div>
            <div className="text-sm">
              <span className="text-muted-foreground">{formatValue(previous, t)}</span>
              {' → '}
              <span className="font-medium">{formatValue(value, t)}</span>
            </div>
          </div>
        );
      })}
    </div>
  );
};
