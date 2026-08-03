import { EventSummary } from '@/components/events/event-summary';
import { Button } from '@/components/ui/button';
import { useServerData } from '@/hooks/use-server-data';
import { type PollActionRes } from '@/types/poll-action.types';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import { ProposalActionAccordion } from './proposal-action-accordion';

export const ProposalActionEvent = ({ action }: { action: PollActionRes }) => {
  const { t } = useTranslation();
  const { serverPath } = useServerData();
  if (!action.event) return null;
  return (
    <ProposalActionAccordion
      value="event"
      ariaLabel={t('proposals.labels.plannedEvent', {
        name: action.event.name,
      })}
      summary={
        <>
          {t('proposals.labels.eventProposal')}:{' '}
          <span className="font-normal">{action.event.name}</span>
        </>
      }
    >
      <div className="sm:col-span-2">
        <EventSummary {...action.event} embedded />
        {action.event.createdEventId && (
          <Button asChild className="mt-4" size="sm">
            <Link to={`${serverPath}/events/${action.event.createdEventId}`}>
              {t('events.actions.viewEvent')}
            </Link>
          </Button>
        )}
      </div>
    </ProposalActionAccordion>
  );
};
