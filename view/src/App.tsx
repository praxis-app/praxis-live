import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { LoginForm } from "@/components/auth/login-form";
import { SignupForm } from "@/components/auth/signup-form";
import { Card, CardContent } from "@/components/ui/card";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";

type HealthResponse = {
  status: string;
};

type User = {
  id: number;
  email: string;
  name: string;
};

type SessionResponse = {
  user: User | null;
};

type ApiError = {
  error?: string;
};

type LoginPayload = {
  email: string;
  password: string;
};

type SignupPayload = {
  email: string;
  name: string;
  password: string;
};

type Notice =
  | {
      kind: "error" | "success";
      message: string;
    }
  | null;

const sessionQueryKey = ["auth", "session"] as const;

async function requestJson<T>(
  path: string,
  init?: RequestInit,
): Promise<T> {
  const response = await fetch(path, {
    credentials: "same-origin",
    headers: {
      Accept: "application/json",
      ...(init?.body ? { "Content-Type": "application/json" } : {}),
      ...init?.headers,
    },
    ...init,
  });

  if (!response.ok) {
    let message = `Request failed with status ${response.status}.`;

    try {
      const error = (await response.json()) as ApiError;

      if (typeof error.error === "string" && error.error.length > 0) {
        message = error.error;
      }
    } catch {
      // Fall back to the generic message when the error body is absent.
    }

    throw new Error(message);
  }

  return response.json() as Promise<T>;
}

function statusLabel(
  health: ReturnType<typeof useHealthQuery>,
): { label: string; tone: "healthy" | "warning" | "loading" } {
  if (health.isPending) {
    return { label: "Checking backend", tone: "loading" };
  }

  if (health.error) {
    return { label: "Backend offline", tone: "warning" };
  }

  return { label: "Backend online", tone: "healthy" };
}

function useHealthQuery() {
  return useQuery({
    queryKey: ["health"],
    queryFn: () => requestJson<HealthResponse>("/api/health"),
  });
}

function useSessionQuery() {
  return useQuery({
    queryKey: sessionQueryKey,
    queryFn: () => requestJson<SessionResponse>("/api/auth/me"),
  });
}

function App() {
  const queryClient = useQueryClient();
  const healthQuery = useHealthQuery();
  const sessionQuery = useSessionQuery();

  const [mode, setMode] = useState<"login" | "signup">("login");
  const [notice, setNotice] = useState<Notice>(null);
  const [loginForm, setLoginForm] = useState<LoginPayload>({
    email: "",
    password: "",
  });
  const [signupForm, setSignupForm] = useState<SignupPayload>({
    email: "",
    name: "",
    password: "",
  });

  const loginMutation = useMutation({
    mutationFn: (payload: LoginPayload) =>
      requestJson<SessionResponse>("/api/auth/login", {
        method: "POST",
        body: JSON.stringify(payload),
      }),
    onSuccess: (data) => {
      queryClient.setQueryData(sessionQueryKey, data);
      setLoginForm((current) => ({ ...current, password: "" }));
      setNotice({
        kind: "success",
        message: `Signed in as ${data.user?.name ?? "your account"}.`,
      });
    },
    onError: (error) => {
      setNotice({
        kind: "error",
        message:
          error instanceof Error ? error.message : "Could not sign in right now.",
      });
    },
  });

  const signupMutation = useMutation({
    mutationFn: (payload: SignupPayload) =>
      requestJson<SessionResponse>("/api/auth/signup", {
        method: "POST",
        body: JSON.stringify(payload),
      }),
    onSuccess: (data) => {
      queryClient.setQueryData(sessionQueryKey, data);
      setSignupForm({ email: "", name: "", password: "" });
      setNotice({
        kind: "success",
        message: `Account created for ${data.user?.name ?? "your workspace"}.`,
      });
    },
    onError: (error) => {
      setNotice({
        kind: "error",
        message:
          error instanceof Error ? error.message : "Could not create the account.",
      });
    },
  });

  const logoutMutation = useMutation({
    mutationFn: () =>
      requestJson<SessionResponse>("/api/auth/logout", {
        method: "POST",
      }),
    onSuccess: (data) => {
      queryClient.setQueryData(sessionQueryKey, data);
      setNotice({
        kind: "success",
        message: "You have been signed out.",
      });
    },
    onError: (error) => {
      setNotice({
        kind: "error",
        message:
          error instanceof Error ? error.message : "Could not sign out right now.",
      });
    },
  });

  const healthState = statusLabel(healthQuery);
  const user = sessionQuery.data?.user ?? null;
  const authBusy =
    loginMutation.isPending || signupMutation.isPending || logoutMutation.isPending;

  function switchMode(nextMode: "login" | "signup") {
    setMode(nextMode);
    setNotice(null);
  }

  function submitLogin() {
    setNotice(null);
    loginMutation.mutate(loginForm);
  }

  function submitSignup() {
    setNotice(null);
    signupMutation.mutate(signupForm);
  }

  return (
    <main className="min-h-screen bg-[radial-gradient(circle_at_top_left,rgba(253,224,71,0.24),transparent_30%),radial-gradient(circle_at_bottom_right,rgba(45,212,191,0.22),transparent_34%),linear-gradient(180deg,rgba(250,250,249,0.96),rgba(244,244,245,1))] px-6 py-10">
      <div className="mx-auto grid w-full max-w-6xl gap-6 lg:grid-cols-[1.15fr_0.85fr]">
        <section className="flex flex-col justify-between gap-6">
          <div className="space-y-6">
            <div className="inline-flex items-center rounded-full border border-border/60 bg-background/80 px-3 py-1 text-xs font-medium uppercase tracking-[0.22em] text-muted-foreground shadow-sm backdrop-blur">
              Praxis Live Auth
            </div>

            <div className="space-y-4">
              <h1 className="max-w-2xl text-4xl font-semibold tracking-tight text-foreground sm:text-5xl">
                Session-backed sign up, login, and logout are wired into the Rust
                API.
              </h1>
              <p className="max-w-xl text-base leading-7 text-muted-foreground sm:text-lg">
                The frontend keeps auth state in React Query, the backend hashes
                passwords with a crate, and login state is stored in a session
                cookie.
              </p>
            </div>
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <Card className="border-border/70 bg-background/75 backdrop-blur">
              <CardContent className="space-y-3 p-6">
                <p className="text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
                  API status
                </p>
                <div className="flex items-center gap-3">
                  {healthQuery.isPending ? <Spinner /> : null}
                  <p className="text-lg font-medium text-foreground">
                    {healthState.label}
                  </p>
                </div>
                <p className="text-sm leading-6 text-muted-foreground">
                  {healthQuery.error instanceof Error
                    ? healthQuery.error.message
                    : "Health checks are responding and ready for auth requests."}
                </p>
              </CardContent>
            </Card>

            <Card className="border-border/70 bg-background/75 backdrop-blur">
              <CardContent className="space-y-3 p-6">
                <p className="text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
                  Session state
                </p>
                <p className="text-lg font-medium text-foreground">
                  {sessionQuery.isPending
                    ? "Resolving current session"
                    : user
                      ? `Signed in as ${user.name}`
                      : "No active session"}
                </p>
                <p className="text-sm leading-6 text-muted-foreground">
                  {user
                    ? user.email
                    : "Create an account or sign in to verify the flow end to end."}
                </p>
              </CardContent>
            </Card>
          </div>
        </section>

        <Card className="border-border/70 bg-background/85 shadow-xl shadow-black/5 backdrop-blur">
          <CardContent className="space-y-6 p-6 sm:p-8">
            <div className="flex items-center justify-between gap-4">
              <div>
                <p className="text-sm font-medium text-muted-foreground">
                  Account access
                </p>
                <h2 className="text-2xl font-semibold tracking-tight text-foreground">
                  {user ? "Authenticated session" : "Get started"}
                </h2>
              </div>

              <div
                className={cn(
                  "rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em]",
                  healthState.tone === "healthy" &&
                    "bg-emerald-100 text-emerald-700",
                  healthState.tone === "loading" &&
                    "bg-amber-100 text-amber-700",
                  healthState.tone === "warning" &&
                    "bg-rose-100 text-rose-700",
                )}
              >
                {healthState.label}
              </div>
            </div>

            {notice ? (
              <div
                className={cn(
                  "rounded-lg border px-4 py-3 text-sm",
                  notice.kind === "success"
                    ? "border-emerald-200 bg-emerald-50 text-emerald-800"
                    : "border-rose-200 bg-rose-50 text-rose-800",
                )}
              >
                {notice.message}
              </div>
            ) : null}

            {sessionQuery.isPending ? (
              <div className="flex min-h-56 items-center justify-center gap-3 rounded-xl border border-dashed border-border/70 bg-muted/20">
                <Spinner className="size-5" />
                <span className="text-sm text-muted-foreground">
                  Loading current session...
                </span>
              </div>
            ) : user ? (
              <div className="space-y-6">
                <div className="rounded-2xl border border-border/70 bg-muted/20 p-5">
                  <p className="text-sm text-muted-foreground">Signed in user</p>
                  <p className="mt-2 text-2xl font-semibold text-foreground">
                    {user.name}
                  </p>
                  <p className="mt-1 text-sm text-muted-foreground">{user.email}</p>
                </div>

                <div className="grid gap-3 rounded-2xl border border-border/70 bg-background/70 p-5 text-sm text-muted-foreground">
                  <p>
                    The session is server-backed, so refreshing the page should keep
                    you signed in until you log out or the server restarts.
                  </p>
                  <p>
                    This first pass stores users and sessions in memory. It proves
                    the auth flow without introducing database schema work yet.
                  </p>
                </div>

                <button
                  className="inline-flex w-full items-center justify-center rounded-lg bg-foreground px-4 py-2.5 text-sm font-medium text-background transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
                  disabled={logoutMutation.isPending}
                  onClick={() => {
                    setNotice(null);
                    logoutMutation.mutate();
                  }}
                  type="button"
                >
                  {logoutMutation.isPending ? (
                    <span className="flex items-center gap-2">
                      <Spinner className="text-background" />
                      Signing out...
                    </span>
                  ) : (
                    "Log out"
                  )}
                </button>
              </div>
            ) : (
              <div className="space-y-6">
                <div className="grid grid-cols-2 rounded-xl border border-border/70 bg-muted/20 p-1">
                  <button
                    className={cn(
                      "rounded-lg px-4 py-2 text-sm font-medium transition",
                      mode === "login"
                        ? "bg-background text-foreground shadow-sm"
                        : "text-muted-foreground hover:text-foreground",
                    )}
                    onClick={() => switchMode("login")}
                    type="button"
                  >
                    Log in
                  </button>
                  <button
                    className={cn(
                      "rounded-lg px-4 py-2 text-sm font-medium transition",
                      mode === "signup"
                        ? "bg-background text-foreground shadow-sm"
                        : "text-muted-foreground hover:text-foreground",
                    )}
                    onClick={() => switchMode("signup")}
                    type="button"
                  >
                    Sign up
                  </button>
                </div>

                {mode === "login" ? (
                  <LoginForm
                    disabled={authBusy}
                    email={loginForm.email}
                    isPending={loginMutation.isPending}
                    onEmailChange={(email) =>
                      setLoginForm((current) => ({ ...current, email }))
                    }
                    onPasswordChange={(password) =>
                      setLoginForm((current) => ({ ...current, password }))
                    }
                    onSubmit={submitLogin}
                    password={loginForm.password}
                  />
                ) : (
                  <SignupForm
                    disabled={authBusy}
                    email={signupForm.email}
                    isPending={signupMutation.isPending}
                    name={signupForm.name}
                    onEmailChange={(email) =>
                      setSignupForm((current) => ({ ...current, email }))
                    }
                    onNameChange={(name) =>
                      setSignupForm((current) => ({ ...current, name }))
                    }
                    onPasswordChange={(password) =>
                      setSignupForm((current) => ({ ...current, password }))
                    }
                    onSubmit={submitSignup}
                    password={signupForm.password}
                  />
                )}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </main>
  );
}

export default App;
