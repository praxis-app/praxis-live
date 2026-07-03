import { cn } from '@/lib/shared.utils';

interface Props {
  label: string;
  value: string;
  met: boolean;
}

export const ProposalRuleRow = ({ label, value, met }: Props) => (
  <div className="flex items-center justify-between gap-4">
    <div>
      <div className="font-medium">{label}</div>
      <div className="text-muted-foreground">{value}</div>
    </div>
    <span
      className={cn(
        met ? 'text-green-600 dark:text-green-400' : 'text-destructive',
      )}
      aria-label={met ? 'met' : 'not met'}
    >
      {met ? '✓' : '✕'}
    </span>
  </div>
);
