import { LandingProposalVisual } from '@/components/landing/landing-proposal-visual';
import {
  Hash,
  MessageCircle,
  MoreHorizontal,
  Phone,
  ThumbsUp,
  Users,
} from 'lucide-react';

interface Props {
  variant: 'flow' | 'conversation' | 'forum' | 'decision';
}

export const LandingVisual = ({ variant }: Props) => {
  if (variant === 'conversation') {
    return (
      <div className="border-border bg-card rounded-[1.75rem] border p-4 shadow-xl shadow-black/5 sm:p-6">
        <div className="mb-6 flex items-center justify-between border-b pb-4">
          <div className="flex items-center gap-2 font-semibold">
            <Hash className="text-muted-foreground size-5" />
            phonebank
          </div>
          <div className="flex items-center gap-2 rounded-full bg-neutral-950 px-3 py-1.5 text-xs font-semibold text-white dark:bg-white dark:text-neutral-950">
            <Phone className="size-3.5" />
            Join call
          </div>
        </div>
        <div className="space-y-5">
          <div className="flex gap-3">
            <div className="bg-praxis-coral-soft text-praxis-coral flex size-9 shrink-0 items-center justify-center rounded-full text-xs font-bold">
              AR
            </div>
            <div>
              <p className="text-sm font-semibold">Ari</p>
              <p className="text-muted-foreground mt-1 text-sm leading-6">
                We still need six volunteers for Saturday morning. Who can
                make calls?
              </p>
            </div>
          </div>
          <div className="flex gap-3">
            <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-emerald-100 text-xs font-bold text-emerald-800 dark:bg-emerald-950 dark:text-emerald-200">
              MK
            </div>
            <div>
              <p className="text-sm font-semibold">Mika</p>
              <p className="text-muted-foreground mt-1 text-sm leading-6">
                I can take the first shift and help train new callers.
              </p>
            </div>
          </div>
          <div className="border-border text-muted-foreground flex items-center gap-2 rounded-xl border px-4 py-3 text-sm">
            <MessageCircle className="size-4" />
            Message #phonebank
          </div>
        </div>
      </div>
    );
  }

  if (variant === 'forum') {
    return (
      <div className="rounded-[1.75rem] bg-neutral-100 p-4 sm:p-7 dark:bg-white/5">
        <div className="border-border bg-card rounded-2xl border p-5 shadow-lg shadow-black/5 sm:p-6">
          <div className="flex items-start justify-between gap-4">
            <div>
              <span className="bg-praxis-coral-soft text-praxis-coral rounded-full px-2.5 py-1 text-xs font-semibold">
                Open discussion
              </span>
              <h3 className="mt-4 text-lg font-bold">
                Saturday phone bank plan
              </h3>
            </div>
            <MoreHorizontal className="text-muted-foreground size-5 shrink-0" />
          </div>
          <p className="text-muted-foreground mt-3 text-sm leading-6">
            We still need to fill the afternoon shifts, divide up the call
            list, and make sure new volunteers are trained before Saturday.
            What can everyone take on?
          </p>
          <div className="border-border mt-6 flex items-center justify-between border-t pt-4 text-sm">
            <span className="text-muted-foreground flex items-center gap-2">
              <MessageCircle className="size-4" /> 8 replies
            </span>
            <span className="font-medium text-emerald-700 dark:text-emerald-300">
              Active now
            </span>
          </div>
        </div>
        <div className="border-border bg-card/80 mx-6 -mt-1 rounded-b-2xl border border-t-0 p-3 opacity-60" />
      </div>
    );
  }

  if (variant === 'decision') {
    return <LandingProposalVisual />;
  }

  return (
    <div className="relative mx-auto w-full max-w-lg py-7 sm:px-6">
      <div className="border-border bg-card relative rounded-[1.75rem] border p-5 shadow-2xl shadow-black/10 sm:p-7">
        <div className="mb-7 flex items-center justify-between">
          <div className="flex items-center gap-2 text-sm font-semibold">
            <Hash className="text-muted-foreground size-4" /> community
          </div>
          <div className="text-muted-foreground flex items-center gap-1.5 text-xs">
            <Users className="size-4" /> 12 online
          </div>
        </div>
        <div className="space-y-3">
          <div className="bg-muted/70 rounded-2xl p-4">
            <div className="flex items-center gap-2 text-sm font-semibold">
              <MessageCircle className="text-praxis-coral size-4" /> Talk it
              through
            </div>
            <p className="text-muted-foreground mt-2 text-xs leading-5">
              Chat naturally, share context, and jump into a channel call.
            </p>
          </div>
          <div className="flex justify-center">
            <div className="bg-border h-5 w-px" />
          </div>
          <div className="bg-praxis-green-soft dark:bg-praxis-green/20 rounded-2xl p-4">
            <div className="text-praxis-green flex items-center gap-2 text-sm font-semibold dark:text-emerald-300">
              <ThumbsUp className="size-4" /> Decide together
            </div>
            <p className="text-muted-foreground mt-2 text-xs leading-5">
              Focus the discussion, propose a path, vote, and ratify the result.
            </p>
          </div>
        </div>
      </div>
      <div className="bg-praxis-coral absolute top-0 right-0 -z-10 size-32 rounded-full opacity-20 blur-3xl" />
      <div className="bg-praxis-green/20 absolute bottom-0 left-0 -z-10 size-40 rounded-full blur-3xl" />
    </div>
  );
};
