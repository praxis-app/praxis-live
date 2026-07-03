import { type PollActionRes } from '@/types/poll-action.types';
import { ServerConfigChanges } from '../server-config-changes';

export const ProposalActionServerConfig = ({ action }: { action: PollActionRes }) =>
  action.serverConfig ? <ServerConfigChanges changes={action.serverConfig} /> : null;
