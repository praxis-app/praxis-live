import { VOTE_TYPES } from '@/constants/vote.constants';

export type VoteType = (typeof VOTE_TYPES)[number];

export interface VoteRes {
  id: string;
  voteType?: VoteType;
  pollOptionIds?: string[];
}

export interface CreateVoteRes {
  id: string;
  pollId: string;
  userId: string;
  voteType?: VoteType;
  pollOptionIds?: string[];
  isRatifyingVote: boolean;
}

export type UpdateVoteRes = {
  isRatifyingVote: boolean;
};

export interface PollOptionVoterRes {
  id: string;
  name: string;
  displayName?: string;
  profilePicture: { id: string } | null;
}

export interface CreateVoteReq {
  voteType?: VoteType;
  pollOptionIds?: string[];
}

export interface UpdateVoteReq {
  voteType?: VoteType;
  pollOptionIds?: string[];
}
