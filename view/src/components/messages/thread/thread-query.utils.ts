export const getThreadQueryKey = (
  serverId?: string,
  channelId?: string,
  rootMessageId?: string,
  inviteToken?: string | null,
) => [
  'servers',
  serverId,
  'channels',
  channelId,
  'messages',
  rootMessageId,
  'replies',
  ...(inviteToken ? ['invite', inviteToken] : []),
];
