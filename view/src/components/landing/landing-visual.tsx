import {
  ChevronUp,
  CircleCheck,
  Hash,
  ListChecks,
  MessageCircle,
  Minus,
  MoreHorizontal,
  Phone,
  Plus,
  ThumbsUp,
  UserCog,
  Users,
  Vote,
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
            courtyard
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
                What if we make the courtyard accessible before the fall event?
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
                I’m in. Let’s work through the options together.
              </p>
            </div>
          </div>
          <div className="border-border text-muted-foreground flex items-center gap-2 rounded-xl border px-4 py-3 text-sm">
            <MessageCircle className="size-4" />
            Message #courtyard
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
                Courtyard accessibility plan
              </h3>
            </div>
            <MoreHorizontal className="text-muted-foreground size-5 shrink-0" />
          </div>
          <p className="text-muted-foreground mt-3 text-sm leading-6">
            Bring the ideas from chat into one focused thread, compare the
            routes, and capture what the group learns.
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
    return (
      <div className="flex max-w-full min-w-0 gap-4 pt-1">
        <div className="bg-praxis-coral-soft text-praxis-coral mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-full text-lg font-light">
          AR
        </div>
        <div className="max-w-full min-w-0 flex-1">
          <div className="flex min-w-0 items-center gap-1.5 pb-1">
            <span className="font-medium">Ari</span>
            <span className="text-muted-foreground text-sm">just now</span>
          </div>

          <div className="border-border bg-card relative min-w-0 rounded-md border px-3 py-3.5 shadow-xl shadow-black/5 before:absolute before:inset-y-0 before:left-0 before:w-3 before:rounded-l-md before:border-l-3 before:border-l-(--border)">
            <MoreHorizontal className="text-muted-foreground absolute top-3 right-3 size-5" />

            <div className="text-muted-foreground flex min-w-0 flex-col items-start gap-1 pr-8 text-xs font-medium sm:flex-row sm:items-center sm:gap-0 sm:text-sm">
              <span className="flex items-center gap-1.5">
                <UserCog className="size-4" />
                Change role
              </span>
              <span className="flex items-center gap-1.5">
                <span className="hidden px-1.5 sm:inline" aria-hidden="true">
                  ·
                </span>
                <ListChecks className="size-4" />
                Consensus
              </span>
            </div>

            <p className="wrap-break-word pt-3 pb-3 text-sm sm:text-base">
              Give the organizing team what they need to coordinate the fall
              event.
            </p>

            <div className="mb-3 rounded-lg bg-black/2 px-3 dark:bg-black/10">
              <div className="flex min-w-0 items-center gap-2 py-3 text-sm font-semibold">
                <ChevronUp className="size-4 shrink-0" />
                <span className="min-w-0 truncate">
                  Role change proposal:{' '}
                  <span
                    className="mr-1 inline-block size-3.5 rounded-full align-[-1px]"
                    style={{ backgroundColor: '#e91e63' }}
                  />
                  <span className="font-normal">Organizers</span>
                </span>
              </div>
              <div className="grid gap-x-6 gap-y-4 px-1 pb-4 sm:grid-cols-2">
                <div className="min-w-0 space-y-2">
                  <p className="text-xs font-medium sm:text-sm">Name</p>
                  <div
                    className="flex min-w-0 items-center gap-2 border px-1.5 py-1.5 text-xs sm:text-sm"
                    style={{ borderRadius: 4 }}
                  >
                    <span
                      className="flex size-4.5 shrink-0 items-center justify-center bg-[#fee2e2] text-[#b91c1c] dark:bg-[#432d2b] dark:text-[#ff2727]"
                      style={{ borderRadius: 4 }}
                    >
                      <Minus className="size-3.5" />
                    </span>
                    <span className="min-w-0 truncate">Event helpers</span>
                  </div>
                  <div
                    className="flex min-w-0 items-center gap-2 border px-1.5 py-1.5 text-xs sm:text-sm"
                    style={{ borderRadius: 4 }}
                  >
                    <span
                      className="flex size-4.5 shrink-0 items-center justify-center bg-[#dcfce7] text-[#15803d] dark:bg-[#2e4532] dark:text-[#0cff4f]"
                      style={{ borderRadius: 4 }}
                    >
                      <Plus className="size-3.5" />
                    </span>
                    <span className="min-w-0 truncate">Organizers</span>
                  </div>
                </div>
                <div className="min-w-0 space-y-2">
                  <p className="text-xs font-medium sm:text-sm">Permissions</p>
                  <div
                    className="flex min-w-0 items-center gap-2 border px-1.5 py-1.5 text-xs sm:text-sm"
                    style={{ borderRadius: 4 }}
                  >
                    <span
                      className="flex size-4.5 shrink-0 items-center justify-center bg-[#dcfce7] text-[#15803d] dark:bg-[#2e4532] dark:text-[#0cff4f]"
                      style={{ borderRadius: 4 }}
                    >
                      <Plus className="size-3.5" />
                    </span>
                    <span className="min-w-0 truncate">Manage channels</span>
                  </div>
                  <div
                    className="flex min-w-0 items-center gap-2 border px-1.5 py-1.5 text-xs sm:text-sm"
                    style={{ borderRadius: 4 }}
                  >
                    <span
                      className="flex size-4.5 shrink-0 items-center justify-center bg-[#dcfce7] text-[#15803d] dark:bg-[#2e4532] dark:text-[#0cff4f]"
                      style={{ borderRadius: 4 }}
                    >
                      <Plus className="size-3.5" />
                    </span>
                    <span className="min-w-0 truncate">Create invites</span>
                  </div>
                </div>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
              {['Agree', 'Disagree', 'Abstain', 'Block'].map((vote) => (
                <div
                  key={vote}
                  className="bg-background flex h-8 items-center justify-center rounded-md border text-xs font-medium shadow-xs sm:text-sm"
                >
                  {vote}
                </div>
              ))}
            </div>

            <div className="border-border mt-5 flex flex-wrap items-center justify-between gap-2 border-t pt-3">
              <div className="text-muted-foreground flex items-center text-xs sm:text-sm">
                <span className="flex items-center gap-1.5">
                  <Vote className="size-4" /> 7 votes
                </span>
                <span className="px-1.5" aria-hidden="true">
                  ·
                </span>
                <span>∞</span>
              </div>
              <span className="flex items-center gap-1.5 rounded-md border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-xs font-medium text-emerald-700 dark:text-emerald-300">
                <CircleCheck className="size-3" />
                Ratified
              </span>
            </div>
          </div>
        </div>
      </div>
    );
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
