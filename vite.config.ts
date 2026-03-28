import path from "path";
import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, __dirname, "");
  const clientPort = Number.parseInt(env.CLIENT_PORT || "5173", 10);

  return {
    envDir: __dirname,
    root: path.resolve(__dirname, "view"),
    server: {
      port: clientPort,
    },
    plugins: [react(), tailwindcss()],
    resolve: {
      alias: {
        "@": path.resolve(__dirname, "view/src"),
      },
    },
  };
});
