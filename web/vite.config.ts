import { readFile } from "node:fs/promises"
import { resolve } from "node:path"
import { defineConfig, Plugin } from "vite"
import deno from "@deno/vite-plugin"
import preact from "@preact/preset-vite"
import tailwindcss from "@tailwindcss/vite"

const isExampleRoute = (url?: string) => url?.startsWith("/examples/") ?? false

const exampleRouteFallback = (): Plugin => ({
  name: "example-route-fallback",
  enforce: "pre",
  configureServer: (server) => {
    server.middlewares.use(async (request, response, next) => {
      if (request.method !== "GET" || !isExampleRoute(request.url)) {
        next()
        return
      }

      const html = await readFile(resolve("index.html"), "utf8")
      response.setHeader("Content-Type", "text/html")
      response.end(await server.transformIndexHtml(request.url ?? "/", html))
    })
  },
  configurePreviewServer: (server) => {
    server.middlewares.use(async (request, response, next) => {
      if (request.method !== "GET" || !isExampleRoute(request.url)) {
        next()
        return
      }

      response.setHeader("Content-Type", "text/html")
      response.end(await readFile(resolve("dist/index.html"), "utf8"))
    })
  },
})

// https://vite.dev/config/
export default defineConfig({
  appType: "spa",
  plugins: [exampleRouteFallback(), deno(), preact(), tailwindcss()],
})
