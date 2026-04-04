import { spawnSync } from "node:child_process";

const composeArgs = ["compose", "-f", "docker-compose.e2e.yml"];
const appUrl = "http://127.0.0.1:3210/api/health";
const commandEnv = {
  ...process.env,
  PATH: `/opt/homebrew/bin:${process.env.PATH ?? ""}`,
};

function run(command, args) {
  return spawnSync(command, args, {
    stdio: "inherit",
    env: commandEnv,
  });
}

function getExitCode(status) {
  return status ?? 1;
}

function cleanup() {
  return run("docker", [...composeArgs, "down", "-v", "--remove-orphans"]);
}

async function waitForAppReady(timeoutMs) {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    try {
      const response = await fetch(appUrl);

      if (response.ok) {
        return true;
      }
    } catch {
      // Keep polling until the timeout expires.
    }

    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  return false;
}

let exitCode = 0;

try {
  const teardownBeforeStart = cleanup();
  if (teardownBeforeStart.status !== 0) {
    console.error("Failed to reset E2E Docker services before test startup.");
    exitCode = getExitCode(teardownBeforeStart.status);
  }

  if (exitCode === 0) {
    const startup = run("docker", [...composeArgs, "up", "-d", "--build"]);
    if (startup.status !== 0) {
      console.error("Failed to start E2E Docker services.");
      exitCode = getExitCode(startup.status);
    }
  }

  if (exitCode === 0) {
    const appReady = await waitForAppReady(120_000);
    if (!appReady) {
      console.error("Timed out waiting for the E2E app to become ready.");
      exitCode = 1;
    }
  }

  if (exitCode === 0) {
    const testRun = run("npx", ["playwright", "test", ...process.argv.slice(2)]);
    exitCode = getExitCode(testRun.status);
  }
} finally {
  const teardown = cleanup();
  if (teardown.status !== 0) {
    console.error("Failed to clean up E2E Docker services.");
    if (exitCode === 0) {
      exitCode = getExitCode(teardown.status);
    }
  }
}

process.exit(exitCode);
