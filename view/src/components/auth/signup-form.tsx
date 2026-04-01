import type { SyntheticEvent } from "react";
import { AuthField } from "@/components/auth/auth-field";
import { Spinner } from "@/components/ui/spinner";

type SignupFormProps = {
  email: string;
  name: string;
  password: string;
  isPending: boolean;
  disabled?: boolean;
  onEmailChange: (value: string) => void;
  onNameChange: (value: string) => void;
  onPasswordChange: (value: string) => void;
  onSubmit: () => void;
};

function SignupForm({
  email,
  name,
  password,
  isPending,
  disabled,
  onEmailChange,
  onNameChange,
  onPasswordChange,
  onSubmit,
}: SignupFormProps) {
  function handleSubmit(event: SyntheticEvent<HTMLFormElement, SubmitEvent>) {
    event.preventDefault();
    onSubmit();
  }

  return (
    <form className="space-y-4" onSubmit={handleSubmit}>
      <AuthField
        autoComplete="name"
        label="Name"
        onChange={(event) => onNameChange(event.target.value)}
        placeholder="Ada Lovelace"
        type="text"
        value={name}
      />
      <AuthField
        autoComplete="email"
        label="Email"
        onChange={(event) => onEmailChange(event.target.value)}
        placeholder="you@example.com"
        type="email"
        value={email}
      />
      <AuthField
        autoComplete="new-password"
        label="Password"
        minLength={8}
        onChange={(event) => onPasswordChange(event.target.value)}
        placeholder="At least 8 characters"
        type="password"
        value={password}
      />
      <div className="flex justify-end pt-1">
        <button
          className="inline-flex h-9 items-center justify-center rounded-md bg-foreground px-4 text-sm font-medium text-background transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
          disabled={disabled}
          type="submit"
        >
          {isPending ? (
            <span className="flex items-center gap-2">
              <Spinner className="text-background" />
              Creating account...
            </span>
          ) : (
            "Create account"
          )}
        </button>
      </div>
    </form>
  );
}

export { SignupForm };
