import { useEffect, useMemo, useState, type ChangeEvent, type FormEvent } from "react";
import {
  Link,
  Navigate,
  Route,
  Routes,
  useNavigate,
  useParams,
} from "react-router-dom";

import {
  ApiError,
  fetchFeed,
  fetchJoinedChannels,
  fetchSession,
  login,
  logout,
  persistAccessToken,
  readStoredAccessToken,
  sendMessage,
  signUp,
  uploadMessageImage,
} from "./api";
import type { Channel, FeedMessage, PublicUser } from "./types";

const DEFAULT_SERVER_ID = "11111111-1111-1111-1111-111111111111";

type SessionState = {
  token: string | null;
  user: PublicUser | null;
  status: "loading" | "authenticated" | "anonymous";
};

type ChatDataState = {
  channels: Channel[];
  feed: FeedMessage[];
  selectedChannel: Channel | null;
  isLoading: boolean;
  error: string | null;
};

function isApiError(error: unknown): error is ApiError {
  return error instanceof ApiError;
}

function formatTimestamp(value: string) {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(new Date(value));
}

function imageUrl(serverId: string, channelId: string, messageId: string, imageId: string) {
  return `/api/servers/${serverId}/channels/${channelId}/messages/${messageId}/images/${imageId}`;
}

function useThemeMode() {
  const [theme, setTheme] = useState<"light" | "dark">(() => {
    const stored = window.localStorage.getItem("praxis-theme");
    if (stored === "light" || stored === "dark") {
      return stored;
    }

    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  });

  useEffect(() => {
    const root = document.documentElement;
    root.classList.toggle("dark", theme === "dark");
    root.classList.toggle("light", theme === "light");
    window.localStorage.setItem("praxis-theme", theme);
  }, [theme]);

  const toggleTheme = () => {
    setTheme((current) => (current === "dark" ? "light" : "dark"));
  };

  return { theme, toggleTheme };
}

function AppShell() {
  const { theme, toggleTheme } = useThemeMode();
  const [session, setSession] = useState<SessionState>({
    token: readStoredAccessToken(),
    user: null,
    status: "loading",
  });

  useEffect(() => {
    const token = readStoredAccessToken();

    if (!token) {
      setSession({ token: null, user: null, status: "anonymous" });
      return;
    }

    let cancelled = false;

    fetchSession(token)
      .then((response) => {
        if (cancelled) {
          return;
        }

        if (!response.user) {
          persistAccessToken(null);
          setSession({ token: null, user: null, status: "anonymous" });
          return;
        }

        setSession({ token, user: response.user, status: "authenticated" });
      })
      .catch(() => {
        if (cancelled) {
          return;
        }

        persistAccessToken(null);
        setSession({ token: null, user: null, status: "anonymous" });
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const handleAuthenticated = (response: {
    access_token?: string | null;
    user: PublicUser | null;
  }) => {
    if (!response.access_token || !response.user) {
      throw new Error("Authentication response did not include a session.");
    }

    persistAccessToken(response.access_token);
    setSession({
      token: response.access_token,
      user: response.user,
      status: "authenticated",
    });
  };

  const handleLogout = async () => {
    await logout();
    persistAccessToken(null);
    setSession({ token: null, user: null, status: "anonymous" });
  };

  if (session.status === "loading") {
    return (
      <main className="screen centered">
        <div className="auth-card auth-card--compact">
          <div className="auth-card__header">
            <div className="brand-mark">P</div>
            <button className="icon-button" onClick={toggleTheme} type="button">
              {theme === "dark" ? "Light" : "Dark"}
            </button>
          </div>
          <h1 className="auth-card__title">Loading chat</h1>
          <p className="muted">Checking your session and preparing the app shell.</p>
        </div>
      </main>
    );
  }

  return (
    <Routes>
      <Route
        path="/"
        element={
          session.status === "authenticated" ? <Navigate to="/chat" replace /> : <LandingPage />
        }
      />
      <Route
        path="/login"
        element={
          session.status === "authenticated" ? (
            <Navigate to="/chat" replace />
          ) : (
            <AuthPage mode="login" onAuthenticated={handleAuthenticated} />
          )
        }
      />
      <Route
        path="/signup"
        element={
          session.status === "authenticated" ? (
            <Navigate to="/chat" replace />
          ) : (
            <AuthPage mode="signup" onAuthenticated={handleAuthenticated} />
          )
        }
      />
      <Route
        path="/chat"
        element={
          session.status === "authenticated" && session.token && session.user ? (
            <ChatPage session={session} onLogout={handleLogout} />
          ) : (
            <Navigate to="/signup" replace />
          )
        }
      />
      <Route
        path="/chat/:channelId"
        element={
          session.status === "authenticated" && session.token && session.user ? (
            <ChatPage session={session} onLogout={handleLogout} />
          ) : (
            <Navigate to="/signup" replace />
          )
        }
      />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

function LandingPage() {
  const { theme, toggleTheme } = useThemeMode();

  return (
    <main className="screen screen--auth">
      <section className="auth-hero">
        <div className="auth-card auth-card--hero">
          <div className="auth-card__header">
            <div className="brand-lockup">
              <div className="brand-mark">P</div>
              <div>
                <p className="brand-title">Praxis</p>
                <p className="brand-subtitle">Basic Chat Slice</p>
              </div>
            </div>
            <button className="icon-button" onClick={toggleTheme} type="button">
              {theme === "dark" ? "Light" : "Dark"}
            </button>
          </div>

          <h1 className="auth-card__title">Chat-first foundations, closer to the legacy shell.</h1>
          <p className="muted auth-card__text">
            Sign in to the default workspace, browse channels from the left rail, read the current
            feed, and send text or image messages. This keeps the migrated scope narrow while
            preserving the layout direction from the legacy frontend.
          </p>
          <div className="auth-actions">
            <Link className="button button--primary" to="/signup">
              Sign up
            </Link>
            <Link className="button button--secondary" to="/login">
              Log in
            </Link>
          </div>
        </div>
      </section>
    </main>
  );
}

function AuthPage({
  mode,
  onAuthenticated,
}: {
  mode: "login" | "signup";
  onAuthenticated(response: { access_token?: string | null; user: PublicUser | null }): void;
}) {
  const { theme, toggleTheme } = useThemeMode();
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isSignup = mode === "signup";

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);
    setIsSubmitting(true);

    try {
      const response = isSignup
        ? await signUp({ name: name.trim(), email: email.trim(), password })
        : await login({ email: email.trim(), password });
      onAuthenticated(response);
      navigate("/chat", { replace: true });
    } catch (error) {
      setError(isApiError(error) ? error.message : "Something went wrong. Please try again.");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <main className="screen screen--auth centered">
      <div className="auth-layout">
        <section className="auth-card auth-card--hero">
          <div className="auth-card__header">
            <div className="brand-lockup">
              <div className="brand-mark">P</div>
              <div>
                <p className="brand-title">Praxis</p>
                <p className="brand-subtitle">Workspace chat</p>
              </div>
            </div>
            <button className="icon-button" onClick={toggleTheme} type="button">
              {theme === "dark" ? "Light" : "Dark"}
            </button>
          </div>

          <h1 className="auth-card__title">{isSignup ? "Create your account" : "Welcome back"}</h1>
          <p className="muted auth-card__text">
            {isSignup
              ? "Join the default Praxis workspace and land directly in chat."
              : "Use your existing credentials to get back to your channels."}
          </p>
        </section>

        <section className="auth-card auth-card--form">
          <div className="auth-switcher" aria-label="Authentication options">
            <Link
              className={isSignup ? "auth-switcher__link auth-switcher__link--active" : "auth-switcher__link"}
              to="/signup"
            >
              Sign up
            </Link>
            <Link
              className={!isSignup ? "auth-switcher__link auth-switcher__link--active" : "auth-switcher__link"}
              to="/login"
            >
              Log in
            </Link>
          </div>

          <form className="auth-form" onSubmit={handleSubmit}>
            {isSignup ? (
              <label className="field">
                <span>Name</span>
                <input
                  autoComplete="username"
                  name="name"
                  required
                  minLength={2}
                  value={name}
                  onChange={(event) => setName(event.target.value)}
                />
              </label>
            ) : null}

            <label className="field">
              <span>Email</span>
              <input
                autoComplete="email"
                name="email"
                required
                type="email"
                value={email}
                onChange={(event) => setEmail(event.target.value)}
              />
            </label>

            <label className="field">
              <span>Password</span>
              <input
                autoComplete={isSignup ? "new-password" : "current-password"}
                name="password"
                required
                type="password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
              />
            </label>

            {error ? (
              <p className="notice notice--error" role="alert">
                {error}
              </p>
            ) : null}

            <button className="button button--primary auth-submit" disabled={isSubmitting} type="submit">
              {isSubmitting ? "Working..." : isSignup ? "Create account" : "Log in"}
            </button>
          </form>
        </section>
      </div>
    </main>
  );
}

function ChatPage({
  session,
  onLogout,
}: {
  session: SessionState;
  onLogout(): Promise<void>;
}) {
  const { theme, toggleTheme } = useThemeMode();
  const navigate = useNavigate();
  const { channelId } = useParams();
  const [chatData, setChatData] = useState<ChatDataState>({
    channels: [],
    feed: [],
    selectedChannel: null,
    isLoading: true,
    error: null,
  });
  const [messageBody, setMessageBody] = useState("");
  const [selectedFiles, setSelectedFiles] = useState<File[]>([]);
  const [composerError, setComposerError] = useState<string | null>(null);
  const [isSending, setIsSending] = useState(false);

  const token = session.token;
  const user = session.user;

  useEffect(() => {
    const authToken = token;

    if (!authToken) {
      return;
    }

    const tokenForRequest = authToken;

    let cancelled = false;

    async function loadChannels() {
      setChatData((current) => ({ ...current, isLoading: true, error: null }));

      try {
        const { channels } = await fetchJoinedChannels(DEFAULT_SERVER_ID, tokenForRequest);

        if (cancelled) {
          return;
        }

        const nextChannel =
          channels.find((channel) => channel.id === channelId) ?? channels[0] ?? null;

        setChatData((current) => ({
          ...current,
          channels,
          selectedChannel: nextChannel,
          isLoading: false,
          error: null,
        }));

        if (nextChannel && nextChannel.id !== channelId) {
          navigate(`/chat/${nextChannel.id}`, { replace: true });
        }
      } catch (error) {
        if (cancelled) {
          return;
        }

        setChatData((current) => ({
          ...current,
          isLoading: false,
          error: isApiError(error) ? error.message : "Failed to load channels.",
        }));
      }
    }

    void loadChannels();

    return () => {
      cancelled = true;
    };
  }, [channelId, navigate, token]);

  useEffect(() => {
    const selectedChannel = chatData.selectedChannel;
    const serverId = selectedChannel?.server.id;
    const authToken = token;

    if (!authToken || !selectedChannel || !serverId) {
      setChatData((current) => ({ ...current, feed: [] }));
      return;
    }

    const tokenForRequest = authToken;
    const selectedChannelId = selectedChannel.id;
    const serverIdForRequest = serverId;

    let cancelled = false;

    async function loadFeed() {
      setChatData((current) => ({ ...current, isLoading: true, error: null }));

      try {
        const { feed } = await fetchFeed(serverIdForRequest, selectedChannelId, tokenForRequest);

        if (cancelled) {
          return;
        }

        setChatData((current) => ({
          ...current,
          feed,
          isLoading: false,
          error: null,
        }));
      } catch (error) {
        if (cancelled) {
          return;
        }

        setChatData((current) => ({
          ...current,
          isLoading: false,
          error: isApiError(error) ? error.message : "Failed to load messages.",
        }));
      }
    }

    void loadFeed();

    return () => {
      cancelled = true;
    };
  }, [chatData.selectedChannel, token]);

  const orderedFeed = useMemo(() => [...chatData.feed].reverse(), [chatData.feed]);

  const handleFileChange = (event: ChangeEvent<HTMLInputElement>) => {
    const nextFiles = Array.from(event.target.files ?? []);
    setSelectedFiles(nextFiles);
  };

  const handleSendMessage = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (!token || !chatData.selectedChannel) {
      return;
    }

    const trimmedBody = messageBody.trim();

    if (!trimmedBody && selectedFiles.length === 0) {
      return;
    }

    setComposerError(null);
    setIsSending(true);

    try {
      const serverId = chatData.selectedChannel.server.id;
      const { message } = await sendMessage(
        serverId,
        chatData.selectedChannel.id,
        token,
        trimmedBody,
        selectedFiles.length,
      );

      const placeholders = message.images ?? [];

      for (const [index, file] of selectedFiles.entries()) {
        const placeholder = placeholders[index];

        if (!placeholder) {
          break;
        }

        await uploadMessageImage(
          serverId,
          chatData.selectedChannel.id,
          message.id,
          placeholder.id,
          token,
          file,
        );
      }

      const { feed } = await fetchFeed(serverId, chatData.selectedChannel.id, token);
      setChatData((current) => ({ ...current, feed }));
      setMessageBody("");
      setSelectedFiles([]);
    } catch (error) {
      setComposerError(isApiError(error) ? error.message : "Failed to send message.");
    } finally {
      setIsSending(false);
    }
  };

  const handleLogoutClick = async () => {
    await onLogout();
    navigate("/login", { replace: true });
  };

  const hasMessages = orderedFeed.length > 0;

  return (
    <main className="chat-shell">
      <aside className="left-nav">
        <div className="left-nav__header">
          <div className="server-trigger">
            <div className="brand-mark brand-mark--small">P</div>
            <div className="server-trigger__text">
              <strong>Praxis</strong>
              <span>Default workspace</span>
            </div>
          </div>
        </div>

        <nav aria-label="Channels" className="channel-list">
          <p className="channel-list__label">Channels</p>
          {chatData.channels.map((channel) => (
            <Link
              className={
                channel.id === chatData.selectedChannel?.id
                  ? "channel-list__item channel-list__item--active"
                  : "channel-list__item"
              }
              key={channel.id}
              to={`/chat/${channel.id}`}
            >
              <span className="channel-list__hash">#</span>
              <span>{channel.name}</span>
            </Link>
          ))}
        </nav>

        <div className="left-nav__footer">
          <div className="user-chip">
            <div className="user-chip__avatar">{user?.name?.slice(0, 1) ?? "U"}</div>
            <div className="user-chip__body">
              <p className="sidebar__user-name">{user?.name}</p>
              <p className="muted user-chip__meta">{user?.email}</p>
            </div>
          </div>

          <div className="left-nav__actions">
            <button className="icon-button" onClick={toggleTheme} type="button">
              {theme === "dark" ? "Light" : "Dark"}
            </button>
            <button className="icon-button" onClick={handleLogoutClick} type="button">
              Log out
            </button>
          </div>
        </div>
      </aside>

      <section className="channel-pane">
        <header className="channel-pane__header">
          <div className="channel-heading">
            <span className="channel-heading__hash">#</span>
            <h2>{chatData.selectedChannel?.name ?? "No channel"}</h2>
          </div>
          <p className="channel-pane__description muted">
            {chatData.selectedChannel?.description ?? "Basic text and image chat for the default workspace."}
          </p>
        </header>

        {chatData.error ? (
          <div className="chat-alert">
            <p className="notice notice--error" role="alert">
              {chatData.error}
            </p>
          </div>
        ) : null}

        <div aria-label="Message feed" className="message-feed">
          {chatData.isLoading ? (
            <div className="empty-state">
              <p className="eyebrow">Loading</p>
              <p className="muted">Fetching channels and recent messages.</p>
            </div>
          ) : hasMessages ? (
            orderedFeed.map((item) => (
              <article className="message-row" key={item.id}>
                <div className="message-row__avatar">{item.user?.name?.slice(0, 1) ?? "U"}</div>
                <div className="message-row__content">
                  <div className="message-row__meta">
                    <strong>{item.user?.name ?? "Unknown user"}</strong>
                    <span className="muted">{formatTimestamp(item.createdAt)}</span>
                  </div>
                  {item.body ? <p className="message-row__body">{item.body}</p> : null}
                  {item.images?.length ? (
                    <div className="message-row__images">
                      {item.images.map((image) => (
                        <img
                          alt="Uploaded attachment"
                          className="message-row__image"
                          key={image.id}
                          src={imageUrl(
                            chatData.selectedChannel?.server.id ?? DEFAULT_SERVER_ID,
                            chatData.selectedChannel?.id ?? "",
                            item.id,
                            image.id,
                          )}
                        />
                      ))}
                    </div>
                  ) : null}
                </div>
              </article>
            ))
          ) : (
            <div className="empty-state">
              <p className="eyebrow">No messages yet</p>
              <p className="muted">Start the conversation with the first message in this channel.</p>
            </div>
          )}
        </div>

        <form className="composer" onSubmit={handleSendMessage}>
          <div className="composer__surface">
            <label className="composer__field">
              <span className="sr-only">Message</span>
              <textarea
                name="body"
                placeholder={`Message #${chatData.selectedChannel?.name ?? "general"}`}
                rows={3}
                value={messageBody}
                onChange={(event) => setMessageBody(event.target.value)}
              />
            </label>

            <div className="composer__toolbar">
              <label className="composer__attach">
                <span>Add images</span>
                <input accept="image/*" multiple name="images" type="file" onChange={handleFileChange} />
              </label>

              <button
                className="button button--primary composer__send"
                disabled={isSending || (!messageBody.trim() && selectedFiles.length === 0)}
                type="submit"
              >
                {isSending ? "Sending..." : "Send message"}
              </button>
            </div>
          </div>

          {selectedFiles.length ? (
            <div className="attachment-list" aria-label="Selected images">
              {selectedFiles.map((file) => (
                <span className="attachment-pill" key={`${file.name}-${file.size}`}>
                  {file.name}
                </span>
              ))}
            </div>
          ) : null}

          {composerError ? (
            <p className="notice notice--error" role="alert">
              {composerError}
            </p>
          ) : null}

          <div className="composer__actions">
            <p className="muted">Signed in as {user?.name}. Messages support text and image uploads.</p>
          </div>
        </form>
      </section>
    </main>
  );
}

export default AppShell;
