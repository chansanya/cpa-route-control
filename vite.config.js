import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

const defaultCpaTarget = process.env.CPA_DEV_TARGET || "http://127.0.0.1:8317";
let activeCpaTarget = normalizeTarget(defaultCpaTarget);

function normalizeTarget(value) {
  const url = new URL(value);
  if (
    !["http:", "https:"].includes(url.protocol) ||
    url.username ||
    url.password
  ) {
    throw new Error(
      "CPA proxy target must be an HTTP(S) origin without credentials",
    );
  }
  return url.origin;
}

function createDynamicCpaProxy({ stripQuery = false } = {}) {
  let proxyServer;
  return {
    target: defaultCpaTarget,
    changeOrigin: true,
    configure: (proxy) => {
      proxyServer = proxy;
    },
    bypass: (request) => {
      const requestUrl = new URL(request.url || "/", "http://localhost");
      const requestedOrigin = requestUrl.searchParams.get("cpa-origin");
      if (requestedOrigin) activeCpaTarget = normalizeTarget(requestedOrigin);
      if (proxyServer) proxyServer.options.target = activeCpaTarget;
      if (stripQuery) request.url = requestUrl.pathname;
    },
  };
}

export default defineConfig({
  plugins: [vue()],
  server: {
    port: 1420,
    strictPort: true,
    proxy: {
      "/cpa-management": {
        target: defaultCpaTarget,
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/cpa-management/, "/v0/management"),
      },
      "/management.html": createDynamicCpaProxy({ stripQuery: true }),
      "/v0/management": createDynamicCpaProxy(),
      "/v1": createDynamicCpaProxy(),
    },
  },
});
