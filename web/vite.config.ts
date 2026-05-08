import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises"
import type { IncomingMessage, ServerResponse } from "node:http"
import { tmpdir } from "node:os"
import { join, resolve } from "node:path"
import process from "node:process"
import { spawn } from "node:child_process"
import { randomUUID } from "node:crypto"
import { defineConfig, Plugin } from "vite"
import deno from "@deno/vite-plugin"
import preact from "@preact/preset-vite"
import tailwindcss from "@tailwindcss/vite"

interface CommandResult {
  status: number | null
  stdout: string
  stderr: string
  timedOut: boolean
  elapsedMs: number
}

interface RunResponse extends CommandResult {
  ok: boolean
  stage: "compile" | "run"
  compileMs: number
  runMs: number
  totalMs: number
}

const RUN_TIMEOUT_MS = 5_000
const MAX_BODY_BYTES = 1_000_000

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

const localRunApi = (): Plugin => ({
  name: "local-run-api",
  configureServer: (server) => {
    server.middlewares.use(async (request, response, next) => {
      if (request.url !== "/api/run") {
        next()
        return
      }

      if (request.method !== "POST") {
        sendJson(response, 405, { error: "method not allowed" })
        return
      }

      try {
        const body = JSON.parse(await readBody(request)) as { rust?: unknown }
        if (typeof body.rust !== "string") {
          sendJson(response, 400, { error: "rust must be a string" })
          return
        }
        sendJson(response, 200, await compileAndRunRust(body.rust))
      } catch (error) {
        sendJson(response, 500, { error: String(error) })
      }
    })
  },
})

const compileAndRunRust = async (rust: string): Promise<RunResponse> => {
  const dir = await mkdtemp(join(tmpdir(), `rusk-run-${randomUUID()}-`))
  try {
    const sourcePath = join(dir, "main.rs")
    const binaryPath = join(
      dir,
      process.platform === "win32" ? "main.exe" : "main",
    )
    await writeFile(sourcePath, rust)

    const started = performance.now()
    const compile = await runCommand("rustc", [
      "--edition=2024",
      sourcePath,
      "-o",
      binaryPath,
    ], dir)
    if (compile.status !== 0) {
      return {
        ...compile,
        ok: false,
        stage: "compile",
        compileMs: compile.elapsedMs,
        runMs: 0,
        totalMs: performance.now() - started,
      }
    }

    const run = await runCommand(binaryPath, [], dir)
    return {
      ...run,
      ok: run.status === 0 && !run.timedOut,
      stage: "run",
      compileMs: compile.elapsedMs,
      runMs: run.elapsedMs,
      totalMs: performance.now() - started,
    }
  } finally {
    await rm(dir, { recursive: true, force: true })
  }
}

const runCommand = (
  command: string,
  args: string[],
  cwd: string,
): Promise<CommandResult> =>
  new Promise((resolve) => {
    const started = performance.now()
    const child = spawn(command, args, { cwd })
    let stdout = ""
    let stderr = ""
    let timedOut = false
    const timeout = setTimeout(() => {
      timedOut = true
      child.kill("SIGKILL")
    }, RUN_TIMEOUT_MS)

    child.stdout.on("data", (chunk) => stdout += chunk)
    child.stderr.on("data", (chunk) => stderr += chunk)
    child.on("close", (status) => {
      clearTimeout(timeout)
      resolve({
        status,
        stdout,
        stderr,
        timedOut,
        elapsedMs: performance.now() - started,
      })
    })
    child.on("error", (error) => {
      clearTimeout(timeout)
      resolve({
        status: null,
        stdout,
        stderr: String(error),
        timedOut,
        elapsedMs: performance.now() - started,
      })
    })
  })

const readBody = (request: IncomingMessage): Promise<string> =>
  new Promise((resolve, reject) => {
    let body = ""
    request.setEncoding("utf8")
    request.on("data", (chunk: string) => {
      body += chunk
      if (body.length > MAX_BODY_BYTES) {
        reject(new Error("request body too large"))
        request.destroy()
      }
    })
    request.on("end", () => resolve(body))
    request.on("error", reject)
  })

const sendJson = (response: ServerResponse, status: number, body: unknown) => {
  response.statusCode = status
  response.setHeader("Content-Type", "application/json")
  response.end(JSON.stringify(body))
}

// https://vite.dev/config/
export default defineConfig({
  appType: "spa",
  resolve: { dedupe: ["preact", "preact/hooks", "preact/jsx-runtime"] },
  plugins: [
    localRunApi(),
    exampleRouteFallback(),
    deno(),
    preact(),
    tailwindcss(),
  ],
})
