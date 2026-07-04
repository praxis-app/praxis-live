import { cn } from '@/lib/shared.utils';
import { type ReactNode } from 'react';
import { LuMinus, LuPlus } from 'react-icons/lu';

interface ChangeValueProps {
  changeType: 'add' | 'remove';
  children: ReactNode;
}

export const ProposalActionChangeValue = ({
  changeType,
  children,
}: ChangeValueProps) => {
  const isAdd = changeType === 'add';
  const Icon = isAdd ? LuPlus : LuMinus;

  return (
    <div className="flex min-w-0 items-center gap-2 rounded-md border px-2 py-1.5">
      <span
        className={cn(
          'flex size-5 shrink-0 items-center justify-center rounded-sm',
          isAdd
            ? 'bg-green-500/20 text-green-600 dark:text-green-400'
            : 'bg-red-500/20 text-red-600 dark:text-red-400',
        )}
      >
        <Icon className="size-4" aria-hidden="true" />
      </span>
      <div className="min-w-0 truncate">{children}</div>
    </div>
  );
};

interface Props {
  label: string;
  oldValue: ReactNode;
  proposedValue: ReactNode;
}

export const ProposalActionChange = ({
  label,
  oldValue,
  proposedValue,
}: Props) => (
  <div className="min-w-0 space-y-2">
    <div className="font-semibold">{label}</div>
    <ProposalActionChangeValue changeType="remove">
      {oldValue}
    </ProposalActionChangeValue>
    <ProposalActionChangeValue changeType="add">
      {proposedValue}
    </ProposalActionChangeValue>
  </div>
);
