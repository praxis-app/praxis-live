import type { SyntheticEvent } from "react";
import { AuthField } from "@/components/auth/auth-field";
import { Spinner } from "@/components/ui/spinner";

type LoginFormProps = {
  email: string;
  password: string;
  isPending: boolean;
  disabled?: boolean;
  onEmailChange: (value: string) => void;
  onPasswordChange: (value: string) => void;
  onSubmit: () => void;
};

function LoginForm({
  email,
  password,
  isPending,
  disabled,
  onEmailChange,
  onPasswordChange,
  onSubmit,
}: LoginFormProps) {
  function handleSubmit(event: SyntheticEvent<HTMLFormElement, SubmitEvent>) {
    event.preventDefault();
    onSubmit();
  }

  return (
    <form className="space-y-4" onSubmit={handleSubmit}>
      <AuthField
        autoComplete="email"
        label="Email"
        onChange={(event) => onEmailChange(event.target.value)}
        placeholder="you@example.com"
        type="email"
        value={email}
      />
      <AuthField
        autoComplete="current-password"
        label="Password"
        onChange={(event) => onPasswordChange(event.target.value)}
        placeholder="Enter your password"
        type="password"
        value={password}
      />
      <button
        className="inline-flex w-full items-center justify-center rounded-lg bg-foreground px-4 py-2.5 text-sm font-medium text-background transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
        disabled={disabled}
        type="submit"
      >
        {isPending ? (
          <span className="flex items-center gap-2">
            <Spinner className="text-background" />
            Signing in...
          </span>
        ) : (
          "Log in"
        )}
      </button>
    </form>
  );
}

export { LoginForm };
