import { sortConsensusVotesByType, type WithVoteType } from '@/lib/vote.utils';
import { type PollConfigRes } from '@/types/poll.types';
import { type VoteRes } from '@/types/vote.types';

/**
 * Calculate progress percentage toward a required voting threshold
 * @param current Current count
 * @param required Required count to meet threshold
 */
export const getProgressPercentage = (
  current: number,
  required: number,
): number =>
  required > 0 ? Math.min(100, Math.round((current / required) * 100)) : 100;

/**
 * Calculate the required count to meet a given threshold in voting
 * @param memberCount Total eligible voters
 * @param threshold Percentage threshold (e.g., 51 for 51%)
 */
export const getRequiredCount = (
  memberCount: number,
  threshold: number,
): number => {
  return Math.ceil(memberCount * (threshold * 0.01));
};

export interface ProposalRuleStatus {
  totalVotes: number;
  agreements: number;
  disagreements: number;
  abstains: number;
  blocks: number;
  requiredAgreements: number;
  requiredQuorum: number;
  agreementMet: boolean;
  quorumMet: boolean;
  disagreementsMet: boolean;
  abstainsMet: boolean;
  blocksMet: boolean;
  passes: boolean;
}

export const getProposalRuleStatus = (
  votes: VoteRes[],
  config: PollConfigRes,
  memberCount: number,
): ProposalRuleStatus => {
  const typedVotes = votes.filter(
    (vote): vote is VoteRes & WithVoteType => !!vote.voteType,
  );
  const { agreements, disagreements, abstains, blocks } =
    sortConsensusVotesByType(typedVotes);
  const participants = agreements.length + disagreements.length;
  const requiredAgreements = getRequiredCount(
    participants,
    config.agreementThreshold ?? 0,
  );
  const requiredQuorum = getRequiredCount(
    memberCount,
    config.quorumThreshold ?? 0,
  );
  const agreementMet =
    participants > 0 && agreements.length >= requiredAgreements;
  const quorumMet =
    !config.quorumEnabled || typedVotes.length >= requiredQuorum;
  const disagreementsMet =
    disagreements.length <= (config.disagreementsLimit ?? 0);
  const abstainsMet = abstains.length <= (config.abstainsLimit ?? 0);
  const blocksMet = blocks.length === 0;

  const passes =
    config.decisionMakingModel === 'consent'
      ? disagreementsMet && abstainsMet && blocksMet
      : config.decisionMakingModel === 'majority-vote'
        ? agreementMet && quorumMet
        : agreementMet &&
          quorumMet &&
          disagreementsMet &&
          abstainsMet &&
          blocksMet;

  return {
    totalVotes: typedVotes.length,
    agreements: agreements.length,
    disagreements: disagreements.length,
    abstains: abstains.length,
    blocks: blocks.length,
    requiredAgreements,
    requiredQuorum,
    agreementMet,
    quorumMet,
    disagreementsMet,
    abstainsMet,
    blocksMet,
    passes,
  };
};
