import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

type AuthFieldProps = ComponentProps<"input"> & {
  label: string;
};

function AuthField({ className, label, ...props }: AuthFieldProps) {
  return (
    <label className="block space-y-2 text-sm font-medium text-foreground">
      <span>{label}</span>
      <input
        className={cn(
          "w-full rounded-lg border border-border/70 bg-background/90 px-3 py-2.5 text-sm shadow-xs transition outline-none",
          "placeholder:text-muted-foreground/80 focus:border-foreground/30 focus:ring-4 focus:ring-foreground/5",
          className,
        )}
        {...props}
      />
    </label>
  );
}

export { AuthField };
