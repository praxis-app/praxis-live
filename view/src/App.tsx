import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Menu } from "lucide-react";
import { useState } from "react";
import { LoginForm } from "@/components/auth/login-form";
import { SignupForm } from "@/components/auth/signup-form";
import { Card, CardContent } from "@/components/ui/card";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
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
  accessToken?: string | null;
};

type ApiError = {
  error?: string;
};

type Notice = {
  kind: "error" | "success";
  message: string;
} | null;

const sessionQueryKey = ["auth", "session"] as const;
const accessTokenStorageKey = "accessToken";
const emptyLoginForm = { email: "", password: "" };
const emptySignupForm = { email: "", name: "", password: "" };

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const accessToken = window.localStorage.getItem(accessTokenStorageKey);
  const response = await fetch(path, {
    headers: {
      Accept: "application/json",
      ...(accessToken ? { Authorization: `Bearer ${accessToken}` } : {}),
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
  const [loginForm, setLoginForm] = useState(emptyLoginForm);
  const [signupForm, setSignupForm] = useState(emptySignupForm);

  const loginMutation = useMutation({
    mutationFn: (payload: typeof emptyLoginForm) =>
      requestJson<SessionResponse>("/api/auth/login", {
        method: "POST",
        body: JSON.stringify(payload),
      }),
    onSuccess: (data) => {
      if (data.accessToken) {
        window.localStorage.setItem(accessTokenStorageKey, data.accessToken);
      } else {
        window.localStorage.removeItem(accessTokenStorageKey);
      }
      queryClient.setQueryData(sessionQueryKey, data);
      setLoginForm(emptyLoginForm);
      setNotice({
        kind: "success",
        message: `Signed in as ${data.user?.name ?? "your account"}.`,
      });
    },
    onError: (error) => {
      setNotice({
        kind: "error",
        message:
          error instanceof Error
            ? error.message
            : "Could not sign in right now.",
      });
    },
  });

  const signupMutation = useMutation({
    mutationFn: (payload: typeof emptySignupForm) =>
      requestJson<SessionResponse>("/api/auth/signup", {
        method: "POST",
        body: JSON.stringify(payload),
      }),
    onSuccess: (data) => {
      if (data.accessToken) {
        window.localStorage.setItem(accessTokenStorageKey, data.accessToken);
      } else {
        window.localStorage.removeItem(accessTokenStorageKey);
      }
      queryClient.setQueryData(sessionQueryKey, data);
      setSignupForm(emptySignupForm);
      setNotice({
        kind: "success",
        message: `Account created for ${data.user?.name ?? "your workspace"}.`,
      });
    },
    onError: (error) => {
      setNotice({
        kind: "error",
        message:
          error instanceof Error
            ? error.message
            : "Could not create the account.",
      });
    },
  });

  const logoutMutation = useMutation({
    mutationFn: () =>
      requestJson<SessionResponse>("/api/auth/logout", {
        method: "POST",
      }),
    onSuccess: (data) => {
      window.localStorage.removeItem(accessTokenStorageKey);
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
          error instanceof Error
            ? error.message
            : "Could not sign out right now.",
      });
    },
  });

  const user = sessionQuery.data?.user ?? null;
  const backendOffline = healthQuery.error instanceof Error;
  const authBusy =
    loginMutation.isPending ||
    signupMutation.isPending ||
    logoutMutation.isPending;

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
    <main className="min-h-screen bg-background px-4 py-8 sm:px-6">
      <div className="mx-auto flex min-h-[calc(100vh-4rem)] max-w-md flex-col justify-center gap-4">
        <div className="flex items-center gap-3 rounded-lg border border-border bg-card/80 px-4 py-3 shadow-sm">
          <img
            alt="praxis"
            className="size-8 rounded-md"
            height={32}
            src="/assets/images/app-icon.png"
            width={32}
          />
          <span className="text-sm font-medium text-foreground">praxis</span>
        </div>

        <Card className="w-full border-border shadow-sm">
          <CardContent className="space-y-2 p-6">
            <div className="flex items-start justify-between gap-4">
              <div className="space-y-1">
                <h1 className="text-xl font-semibold text-foreground">
                  {user ? user.name : "Welcome"}
                </h1>
                <p className="text-sm text-muted-foreground">
                  {user
                    ? user.email
                    : mode === "login"
                      ? "Log in"
                      : "Create account"}
                </p>
              </div>

              {user ? (
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <button
                      aria-label="Account menu"
                      className="inline-flex size-8 cursor-pointer items-center justify-center rounded-md border border-border bg-muted text-muted-foreground transition hover:bg-accent hover:text-accent-foreground"
                      type="button"
                    >
                      <Menu className="size-4" />
                    </button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem
                      disabled={logoutMutation.isPending}
                      onClick={() => {
                        setNotice(null);
                        logoutMutation.mutate();
                      }}
                    >
                      {logoutMutation.isPending ? "Signing out..." : "Log out"}
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              ) : null}
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

            {backendOffline && healthQuery.error instanceof Error ? (
              <div className="rounded-md border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">
                {healthQuery.error.message}
              </div>
            ) : null}

            {healthQuery.isPending || sessionQuery.isPending ? (
              <div className="flex min-h-32 items-center justify-center gap-2 text-sm text-muted-foreground">
                <Spinner className="size-5" />
                <span>Loading...</span>
              </div>
            ) : user ? (
              <div className="space-y-4" />
            ) : (
              <div className="space-y-4">
                <div className="grid grid-cols-2 rounded-md border border-border p-1">
                  <button
                    className={cn(
                      "rounded-sm px-3 py-2 text-sm transition",
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
                      "rounded-sm px-3 py-2 text-sm transition",
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
