export interface CallRes {
  id: string;
  serverId: string;
  channelId: string;
  roomName: string;
  status: string;
}

export interface JoinCallRes {
  livekitUrl: string;
  roomName: string;
  token: string;
  call: CallRes;
}
