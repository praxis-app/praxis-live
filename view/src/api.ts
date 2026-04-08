import type { Channel, FeedMessage, Message, SessionResponse } from "./types";

const API_ROOT = "/api";

export const ACCESS_TOKEN_KEY = "access_token";

type RequestOptions = {
  method?: string;
  token?: string | null;
  body?: BodyInit | null;
  json?: unknown;
  headers?: HeadersInit;
};

class ApiError extends Error {
  status: number;

  constructor(message: string, status: number) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const headers = new Headers(options.headers);

  if (options.token) {
    headers.set("Authorization", `Bearer ${options.token}`);
  }

  let body = options.body;

  if (options.json !== undefined) {
    headers.set("Content-Type", "application/json");
    body = JSON.stringify(options.json);
  }

  const response = await fetch(`${API_ROOT}${path}`, {
    method: options.method ?? "GET",
    headers,
    body,
  });

  if (!response.ok) {
    const fallbackMessage = `Request failed with status ${response.status}`;

    try {
      const payload = (await response.json()) as { error?: string };
      throw new ApiError(payload.error ?? fallbackMessage, response.status);
    } catch (error) {
      if (error instanceof ApiError) {
        throw error;
      }

      throw new ApiError(fallbackMessage, response.status);
    }
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

export { ApiError };

export function readStoredAccessToken() {
  return window.localStorage.getItem(ACCESS_TOKEN_KEY);
}

export function persistAccessToken(token: string | null) {
  if (token) {
    window.localStorage.setItem(ACCESS_TOKEN_KEY, token);
    return;
  }

  window.localStorage.removeItem(ACCESS_TOKEN_KEY);
}

export function fetchSession(token: string) {
  return request<SessionResponse>("/auth/me", { token });
}

export function signUp(input: { name: string; email: string; password: string }) {
  return request<SessionResponse>("/auth/signup", {
    method: "POST",
    json: input,
  });
}

export function login(input: { email: string; password: string }) {
  return request<SessionResponse>("/auth/login", {
    method: "POST",
    json: input,
  });
}

export function logout() {
  return request<SessionResponse>("/auth/logout", {
    method: "POST",
  });
}

export function fetchJoinedChannels(serverId: string, token: string) {
  return request<{ channels: Channel[] }>(`/servers/${serverId}/channels/joined`, {
    token,
  });
}

export function fetchFeed(serverId: string, channelId: string, token: string) {
  return request<{ feed: FeedMessage[] }>(
    `/servers/${serverId}/channels/${channelId}/feed?offset=0&limit=50`,
    { token },
  );
}

export function sendMessage(
  serverId: string,
  channelId: string,
  token: string,
  body: string,
  imageCount: number,
) {
  return request<{ message: Message }>(`/servers/${serverId}/channels/${channelId}/messages`, {
    method: "POST",
    token,
    json: {
      body,
      imageCount,
    },
  });
}

export function uploadMessageImage(
  serverId: string,
  channelId: string,
  messageId: string,
  imageId: string,
  token: string,
  file: File,
) {
  const formData = new FormData();
  formData.set("file", file);

  return request<{ image: { id: string } }>(
    `/servers/${serverId}/channels/${channelId}/messages/${messageId}/images/${imageId}/upload`,
    {
      method: "POST",
      token,
      body: formData,
    },
  );
}
