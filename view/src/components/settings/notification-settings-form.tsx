import { api } from '@/client/api-client';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { handleError } from '@/lib/error.utils';
import { type UserConfigRes } from '@/types/user-config.types';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';

interface Props {
  userConfig: UserConfigRes;
}

type ToggleName = keyof Omit<UserConfigRes, 'updatedAt'>;

const TOGGLES: ToggleName[] = [
  'messageNotificationsEnabled',
  'replyNotificationsEnabled',
  'proposalNotificationsEnabled',
  'roleNotificationsEnabled',
];

export const NotificationSettingsForm = ({ userConfig }: Props) => {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const { mutate: updateUserConfig, isPending } = useMutation({
    mutationFn: async (name: ToggleName) => {
      const { userConfig: updated } = await api.updateUserConfig({
        [name]: !userConfig[name],
      });
      return updated;
    },
    onSuccess: (updated) => {
      queryClient.setQueryData(['users', 'me', 'configs'], {
        userConfig: updated,
      });
    },
    onError: (error: Error) => {
      handleError(error);
    },
  });

  return (
    <div className="flex flex-col">
      {TOGGLES.map((name) => (
        <div
          key={name}
          className="flex flex-row items-center justify-between border-b py-6 first:pt-1 last:border-b-0 last:pb-1"
        >
          <div className="space-y-0.5 pr-4">
            <Label htmlFor={name}>{t(`settings.names.${name}`)}</Label>
            <p className="text-muted-foreground text-sm">
              {t(`settings.descriptions.${name}`)}
            </p>
          </div>
          <Switch
            id={name}
            checked={userConfig[name]}
            disabled={isPending}
            onCheckedChange={() => updateUserConfig(name)}
            aria-label={t(`settings.names.${name}`)}
          />
        </div>
      ))}
    </div>
  );
};
