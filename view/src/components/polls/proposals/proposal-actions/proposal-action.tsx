import { type PollActionRes } from '@/types/poll-action.types';
import { ProposalActionRole } from './proposal-action-role';
import { ProposalActionServerConfig } from './proposal-action-server-config';

interface Props {
  action: PollActionRes;
}

export const ProposalAction = ({ action }: Props) => {
  if (action.actionType === 'change-settings' && action.serverConfig) {
    return <ProposalActionServerConfig action={action} />;
  }
  if (
    action.serverRole &&
    (action.actionType === 'change-role' || action.actionType === 'create-role')
  ) {
    return <ProposalActionRole action={action} />;
  }

  return null;
};
