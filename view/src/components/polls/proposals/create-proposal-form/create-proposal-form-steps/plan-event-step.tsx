import { RoleMemberOption } from '@/components/roles/role-member-option';
import { type WizardStepProps } from '@/components/shared/wizard/wizard.types';
import { Button } from '@/components/ui/button';
import {
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { useState } from 'react';
import { useFormContext } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { useWizardContext } from '../../../../shared/wizard/wizard-hooks';
import {
  type CreateProposalFormSchema,
  type CreateProposalWizardContext,
} from '../create-proposal-form.types';

export const PlanEventStep = ({ isLoading }: WizardStepProps) => {
  const [searchTerm, setSearchTerm] = useState('');
  const { context, onNext, onPrevious } =
    useWizardContext<CreateProposalWizardContext>();
  const form = useFormContext<CreateProposalFormSchema>();
  const { t } = useTranslation();
  const online = form.watch('eventOnline');
  const hostIds = form.watch('eventHostIds') || [];
  const members = (context.serverMembers || []).filter((user) =>
    (user.displayName || user.name)
      .toLowerCase()
      .includes(searchTerm.toLowerCase()),
  );

  if (isLoading) return <p>{t('actions.loading')}</p>;

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h2 className="text-lg font-semibold">
          {t('proposals.headers.planEvent')}
        </h2>
        <p className="text-muted-foreground text-sm">
          {t('proposals.descriptions.planEvent')}
        </p>
      </div>
      <div className="grid gap-4 sm:grid-cols-2">
        <FormField
          control={form.control}
          name="eventName"
          render={({ field }) => (
            <FormItem className="sm:col-span-2">
              <FormLabel>{t('events.labels.name')}</FormLabel>
              <FormControl>
                <Input {...field} />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />
        <FormField
          control={form.control}
          name="eventDescription"
          render={({ field }) => (
            <FormItem className="sm:col-span-2">
              <FormLabel>{t('events.labels.description')}</FormLabel>
              <FormControl>
                <Textarea {...field} />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />
        <FormField
          control={form.control}
          name="eventStartsAt"
          render={({ field }) => (
            <FormItem>
              <FormLabel>{t('events.labels.startsAt')}</FormLabel>
              <FormControl>
                <Input type="datetime-local" {...field} />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />
        <FormField
          control={form.control}
          name="eventEndsAt"
          render={({ field }) => (
            <FormItem>
              <FormLabel>{t('events.labels.endsAt')}</FormLabel>
              <FormControl>
                <Input type="datetime-local" {...field} />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />
        <FormField
          control={form.control}
          name="eventOnline"
          render={({ field }) => (
            <FormItem className="flex items-center justify-between rounded-lg border p-3 sm:col-span-2">
              <FormLabel>{t('events.labels.online')}</FormLabel>
              <FormControl>
                <Switch
                  checked={field.value}
                  onCheckedChange={field.onChange}
                />
              </FormControl>
            </FormItem>
          )}
        />
        {!online && (
          <FormField
            control={form.control}
            name="eventLocation"
            render={({ field }) => (
              <FormItem className="sm:col-span-2">
                <FormLabel>{t('events.labels.location')}</FormLabel>
                <FormControl>
                  <Input {...field} />
                </FormControl>
                <FormMessage />
              </FormItem>
            )}
          />
        )}
        {online && (
          <FormField
            control={form.control}
            name="eventExternalLink"
            render={({ field }) => (
              <FormItem className="sm:col-span-2">
                <FormLabel>{t('events.labels.externalLink')}</FormLabel>
                <FormControl>
                  <Input type="url" placeholder="https://" {...field} />
                </FormControl>
                <FormMessage />
              </FormItem>
            )}
          />
        )}
      </div>
      <div className="space-y-3">
        <FormLabel>{t('events.labels.hosts')}</FormLabel>
        <Input
          value={searchTerm}
          onChange={(event) => setSearchTerm(event.target.value)}
          placeholder={t('proposals.placeholders.searchMembersPlaceholder')}
        />
        <div className="max-h-64 overflow-y-auto rounded-lg border px-3">
          {members.map((user) => (
            <RoleMemberOption
              key={user.id}
              user={user}
              selectedUserIds={hostIds}
              setSelectedUserIds={(ids) =>
                form.setValue('eventHostIds', ids, {
                  shouldDirty: true,
                  shouldValidate: true,
                })
              }
            />
          ))}
        </div>
        {form.formState.errors.eventHostIds && (
          <p className="text-destructive text-sm">
            {form.formState.errors.eventHostIds.message}
          </p>
        )}
      </div>
      <div className="flex justify-between">
        <Button variant="outline" onClick={onPrevious}>
          {t('actions.previous')}
        </Button>
        <Button onClick={onNext}>{t('actions.next')}</Button>
      </div>
    </div>
  );
};
