import { Time } from '@/constants/shared.constants';
import { timeFromNow } from '@/lib/time.utils';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

const deadlineHasPassed = (closingAt?: string) =>
  !!closingAt && Date.now() >= new Date(closingAt).getTime();

export const useVotingDeadline = (closingAt?: string) => {
  const [hasPassed, setHasPassed] = useState(() =>
    deadlineHasPassed(closingAt),
  );

  useEffect(() => {
    setHasPassed(deadlineHasPassed(closingAt));
    if (!closingAt) return;

    const deadline = new Date(closingAt).getTime();
    let timeout: number | undefined;
    const scheduleDeadline = () => {
      const remaining = deadline - Date.now();
      if (remaining <= 0) {
        setHasPassed(true);
        return;
      }
      timeout = window.setTimeout(
        scheduleDeadline,
        Math.min(remaining, 2_147_483_647),
      );
    };
    scheduleDeadline();
    return () => window.clearTimeout(timeout);
  }, [closingAt]);

  return hasPassed;
};

const getRefreshDelay = (closingAt: string) => {
  const remaining = (new Date(closingAt).getTime() - Date.now()) / 1000;
  if (remaining < Time.Hour) {
    return 10 * 1000;
  }
  if (remaining < Time.Day) {
    return 60 * 1000;
  }
  return 5 * 60 * 1000;
};

/** Live countdown label for a deadline. `label` is null when there's no deadline. */
export const useVotingDeadlineLabel = (
  closingAt?: string,
  isActive = true,
): { hasEnded: boolean; label: string | null } => {
  const [, setTick] = useState(0);

  const hasPassed = useVotingDeadline(closingAt);
  const { t } = useTranslation();

  const hasEnded = !isActive || hasPassed;
  const isCountingDown = !hasEnded && !!closingAt;

  useEffect(() => {
    if (!isCountingDown || !closingAt) {
      return;
    }
    let timeout: number | undefined;
    const scheduleRefresh = () => {
      timeout = window.setTimeout(() => {
        setTick((tick) => tick + 1);
        scheduleRefresh();
      }, getRefreshDelay(closingAt));
    };
    scheduleRefresh();
    return () => window.clearTimeout(timeout);
  }, [isCountingDown, closingAt]);

  if (hasEnded) {
    return { hasEnded, label: t('time.ended') };
  }
  return {
    hasEnded,
    label: closingAt ? timeFromNow(closingAt, true) : null,
  };
};
