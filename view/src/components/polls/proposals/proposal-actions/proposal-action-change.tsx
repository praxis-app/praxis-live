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
    <div className="flex min-w-0 items-center gap-2 rounded-[4px] border px-1.5 py-1.5">
      <span
        className="flex size-4.5 shrink-0 items-center justify-center rounded-[4px]"
        style={{
          backgroundColor: isAdd ? '#2e4532' : '#432d2b',
          color: isAdd ? '#0cff4f' : '#ff2727',
        }}
      >
        <Icon className="size-3.5" aria-hidden="true" />
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
