import {
  Check,
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
            courtyard
          </div>
          <div className="bg-blurple-1 flex items-center gap-2 rounded-full px-3 py-1.5 text-xs font-semibold text-white">
            <Phone className="size-3.5" />
            Join call
          </div>
        </div>
        <div className="space-y-5">
          <div className="flex gap-3">
            <div className="bg-blurple-1/20 text-blurple-2 flex size-9 shrink-0 items-center justify-center rounded-full text-xs font-bold">
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
      <div className="bg-blurple-1/8 rounded-[1.75rem] p-4 sm:p-7">
        <div className="border-border bg-card rounded-2xl border p-5 shadow-lg shadow-black/5 sm:p-6">
          <div className="flex items-start justify-between gap-4">
            <div>
              <span className="bg-blurple-1/10 text-blurple-2 dark:text-blurple-3 rounded-full px-2.5 py-1 text-xs font-semibold">
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
      <div className="border-border bg-card rounded-[1.75rem] border p-5 shadow-xl shadow-black/5 sm:p-7">
        <div className="flex items-center justify-between gap-4">
          <div>
            <p className="text-blurple-2 dark:text-blurple-3 text-xs font-semibold tracking-wide uppercase">
              Consent proposal
            </p>
            <h3 className="mt-2 text-lg font-bold">
              Approve the accessible route
            </h3>
          </div>
          <div className="flex size-10 items-center justify-center rounded-full bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300">
            <Check className="size-5" />
          </div>
        </div>
        <div className="mt-7 space-y-4">
          <div>
            <div className="mb-2 flex justify-between text-sm">
              <span className="font-medium">Agreement</span>
              <span className="text-muted-foreground">7 of 8</span>
            </div>
            <div className="bg-muted h-2.5 overflow-hidden rounded-full">
              <div className="bg-blurple-1 h-full w-[87.5%] rounded-full" />
            </div>
          </div>
          <div className="grid grid-cols-3 gap-2 text-center text-xs font-medium">
            <div className="rounded-xl bg-emerald-100 px-2 py-3 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-200">
              Agree · 7
            </div>
            <div className="bg-muted rounded-xl px-2 py-3">Abstain · 1</div>
            <div className="bg-muted rounded-xl px-2 py-3">Block · 0</div>
          </div>
          <div className="border-border flex items-center gap-2 border-t pt-4 text-sm font-semibold text-emerald-700 dark:text-emerald-300">
            <Check className="size-4" /> Decision ratified
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="relative mx-auto w-full max-w-lg py-7 sm:px-6">
      <div className="border-border bg-card relative rounded-[1.75rem] border p-5 shadow-2xl shadow-indigo-950/10 sm:p-7">
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
              <MessageCircle className="text-blurple-2 size-4" /> Talk it
              through
            </div>
            <p className="text-muted-foreground mt-2 text-xs leading-5">
              Chat naturally, share context, and jump into a channel call.
            </p>
          </div>
          <div className="flex justify-center">
            <div className="bg-border h-5 w-px" />
          </div>
          <div className="bg-blurple-1/10 rounded-2xl p-4">
            <div className="text-blurple-2 dark:text-blurple-3 flex items-center gap-2 text-sm font-semibold">
              <ThumbsUp className="size-4" /> Decide together
            </div>
            <p className="text-muted-foreground mt-2 text-xs leading-5">
              Focus the discussion, propose a path, vote, and ratify the result.
            </p>
          </div>
        </div>
      </div>
      <div className="bg-blurple-1 absolute top-0 right-0 -z-10 size-32 rounded-full opacity-20 blur-3xl" />
      <div className="absolute bottom-0 left-0 -z-10 size-40 rounded-full bg-emerald-400/20 blur-3xl" />
    </div>
  );
};
