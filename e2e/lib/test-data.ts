import { randomUUID } from "node:crypto";

export const ACCESS_TOKEN_KEY = "access_token";
export const DEFAULT_SERVER_ID = "11111111-1111-1111-1111-111111111111";
export const TEST_PASSWORD = "Password123!";

export type TestUser = {
  name: string;
  email: string;
  password: string;
  suffix: string;
};

export function createTestUser(label = "user"): TestUser {
  const suffix = randomUUID().slice(0, 8);

  return {
    name: `E2E ${label} ${suffix}`,
    email: `e2e_${label}_${suffix}@example.com`,
    password: TEST_PASSWORD,
    suffix,
  };
}

export function createTestMessage(label: string, suffix: string) {
  return `E2E ${label} message ${suffix}`;
}
