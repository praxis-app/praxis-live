import { LoaderCircle } from "lucide-react";
import { cn } from "@/lib/utils";

type SpinnerProps = {
  className?: string;
};

function Spinner({ className }: SpinnerProps) {
  return (
    <LoaderCircle
      aria-hidden="true"
      className={cn("size-4 animate-spin text-muted-foreground", className)}
    />
  );
}

export { Spinner };
