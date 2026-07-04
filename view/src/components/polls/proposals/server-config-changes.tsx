import { type PollActionServerConfigRes } from '@/types/poll-action.types';
import { useTranslation } from 'react-i18next';
import { ProposalActionChange } from './proposal-actions/proposal-action-change';
import { SERVER_CONFIG_FIELDS } from './server-config-changes.utils';

const fields = SERVER_CONFIG_FIELDS;

const formatValue = (
  field: (typeof fields)[number],
  value: unknown,
  t: (key: string) => string,
) => {
  if (typeof value === 'boolean') {
    return t(value ? 'actions.enabled' : 'actions.disabled');
  }
  if (field === 'agreementThreshold' || field === 'quorumThreshold') {
    return `${String(value)}%`;
  }
  if (field === 'decisionMakingModel') {
    return t(
      {
        consent: 'proposals.labels.consent',
        consensus: 'proposals.labels.consensus',
        'majority-vote': 'proposals.labels.majority',
      }[String(value)] ?? String(value),
    );
  }
  return String(value);
};

export const ServerConfigChanges = ({
  changes,
}: {
  changes: PollActionServerConfigRes;
}) => {
  const { t } = useTranslation();
  return (
    <div className="col-span-full grid gap-x-6 gap-y-4 sm:grid-cols-2">
      {fields.map((field) => {
        const value = changes[field];
        const previous =
          changes[
            `prev${field[0].toUpperCase()}${field.slice(1)}` as keyof PollActionServerConfigRes
          ];
        if (value === undefined || previous === undefined) return null;
        return (
          <ProposalActionChange
            key={field}
            label={t(`settings.names.${field}`)}
            oldValue={formatValue(field, previous, t)}
            proposedValue={formatValue(field, value, t)}
          />
        );
      })}
    </div>
  );
};
