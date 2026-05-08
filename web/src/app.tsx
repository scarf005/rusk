import { useEffect, useRef } from "preact/hooks"
import { computed, signal, useSignal } from "@preact/signals"
import { useLocation } from "wouter-preact"
import {
  format_rusk,
  transpile_syntax_tree_json,
  transpile_to_rust,
} from "./wasm/rusk.js"
import { InputEditor, OutputDisplay } from "./Editor.tsx"
import { Header, Layout, Main, Panel } from "./Layout.tsx"
import {
  DEFAULT_EXAMPLE_NAME,
  EXAMPLE_NAMES,
  ExampleName,
  exampleNameFromPath,
  examplePath,
  exampleSource,
} from "./constants.ts"

type OutputMode = "rust" | "syntax-tree" | "run"
type RunState = "idle" | "running"

type RunResponse = {
  ok: boolean
  stage: "compile" | "run"
  status: number | null
  stdout: string
  stderr: string
  timedOut: boolean
}

const isDevServer =
  (import.meta as ImportMeta & { env: { DEV: boolean } }).env.DEV

const selectedExample = signal<ExampleName>(DEFAULT_EXAMPLE_NAME)
const inputCode = signal<string>(exampleSource(DEFAULT_EXAMPLE_NAME))
const outputMode = signal<OutputMode>("rust")
const runOutput = signal("")
const runState = signal<RunState>("idle")

const transpiled = computed(() => {
  const started = performance.now()
  try {
    const rustStarted = performance.now()
    const rust = transpile_to_rust(inputCode.value)
    const rustMs = performance.now() - rustStarted
    const syntaxTreeStarted = performance.now()
    const syntaxTree = transpile_syntax_tree_json(inputCode.value)
    const syntaxTreeMs = performance.now() - syntaxTreeStarted
    return { rust, syntaxTree, rustMs, syntaxTreeMs, error: "" }
  } catch (error) {
    const elapsedMs = performance.now() - started
    return {
      rust: "",
      syntaxTree: "",
      rustMs: elapsedMs,
      syntaxTreeMs: elapsedMs,
      error: stringifyError(error),
    }
  }
})

const outputText = computed(() => {
  if (transpiled.value.error) {
    return "// Fix the Rusk source to see generated output."
  }
  if (outputMode.value === "run") return runOutput.value
  return outputMode.value === "rust"
    ? transpiled.value.rust
    : transpiled.value.syntaxTree
})

const stats = computed(() => ({
  sourceLines: countLines(inputCode.value),
  outputLines: countLines(outputText.value),
  outputMs: outputMode.value === "rust"
    ? transpiled.value.rustMs
    : outputMode.value === "syntax-tree"
    ? transpiled.value.syntaxTreeMs
    : 0,
}))

export function App() {
  const [location, navigate] = useLocation()
  const copied = useSignal(false)
  const inputRef = useRef<HTMLElement>(null)
  const outputRef = useRef<HTMLElement>(null)
  const isScrollingRef = useRef(false)

  const showExample = (name: ExampleName) => {
    selectedExample.value = name
    inputCode.value = exampleSource(name)
    inputRef.current?.scrollTo({ top: 0 })
    outputRef.current?.scrollTo({ top: 0 })
  }

  useEffect(() => {
    const path = location === "/" ? globalThis.location.pathname : location
    const name = path === "/" ? DEFAULT_EXAMPLE_NAME : exampleNameFromPath(path)

    if (!name) {
      navigate(examplePath(DEFAULT_EXAMPLE_NAME))
      return
    }

    if (path === "/") {
      navigate(examplePath(name))
      return
    }

    if (
      selectedExample.value === name &&
      inputCode.value === exampleSource(name)
    ) return

    showExample(name)
  }, [location, navigate])

  const handleScroll = (source: "input" | "output") => {
    if (isScrollingRef.current) return

    const inputEl = inputRef.current
    const outputEl = outputRef.current
    if (!inputEl || !outputEl) return

    isScrollingRef.current = true
    if (source === "input") {
      const ratio = inputEl.scrollTop /
        (inputEl.scrollHeight - inputEl.clientHeight || 1)
      outputEl.scrollTop = ratio *
        (outputEl.scrollHeight - outputEl.clientHeight)
    } else {
      const ratio = outputEl.scrollTop /
        (outputEl.scrollHeight - outputEl.clientHeight || 1)
      inputEl.scrollTop = ratio * (inputEl.scrollHeight - inputEl.clientHeight)
    }
    setTimeout(() => isScrollingRef.current = false, 10)
  }

  const loadExample = (name: ExampleName) => {
    showExample(name)
    const path = examplePath(name)
    if (location === path) return
    navigate(path)
  }

  const copyOutput = async () => {
    await navigator.clipboard.writeText(outputText.value)
    copied.value = true
    setTimeout(() => copied.value = false, 900)
  }

  const formatInput = () => inputCode.value = format_rusk(inputCode.value, 100)

  const runRust = async () => {
    if (transpiled.value.error || runState.value === "running") return

    runState.value = "running"
    outputMode.value = "run"
    runOutput.value = "Running..."
    try {
      const response = await fetch("/api/run", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ rust: transpiled.value.rust }),
      })
      const result = await response.json() as RunResponse | { error: string }
      runOutput.value = "error" in result
        ? result.error
        : formatRunOutput(result)
    } catch (error) {
      runOutput.value = stringifyError(error)
    } finally {
      runState.value = "idle"
    }
  }

  return (
    <Layout>
      <Header>
        <div class="flex-none flex items-center px-4 py-3 border-b-2 lg:border-b-0 lg:border-r-2 border-black min-w-[200px] bg-black text-white">
          <h1 class="text-lg font-bold tracking-tighter uppercase">
            RUSK DEMO
          </h1>
        </div>

        <div class="flex-1 flex overflow-x-auto w-full border-b-2 lg:border-b-0 lg:border-r-2 border-black no-scrollbar bg-white">
          {([
            "rust",
            "syntax-tree",
            ...(runOutput.value ? ["run"] : []),
          ] as OutputMode[]).map((mode) => (
            <button
              type="button"
              key={mode}
              onClick={() => outputMode.value = mode as OutputMode}
              class={`flex-none px-4 md:px-6 py-3 font-bold text-xs md:text-sm uppercase transition-none border-r-2 border-black focus:outline-none whitespace-nowrap ${
                outputMode.value === mode
                  ? "bg-black text-white"
                  : "bg-white text-black hover:bg-gray-200"
              }`}
            >
              {mode === "rust"
                ? "Rust Output"
                : mode === "syntax-tree"
                ? "Syntax Tree"
                : "Run Output"}
            </button>
          ))}
        </div>

        <div class="flex-none w-full lg:w-auto flex flex-row border-b-2 lg:border-b-0 border-black bg-white">
          <div class="flex-1 lg:flex-none relative border-r-2 border-black">
            <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
              <span class="text-[10px] md:text-xs font-bold uppercase mr-2 opacity-50">
                EXPL:
              </span>
            </div>
            <select
              value={selectedExample.value}
              onChange={(event) =>
                loadExample(event.currentTarget.value as ExampleName)}
              class="appearance-none w-full lg:w-[210px] bg-white pl-14 pr-8 py-3 text-xs md:text-sm font-bold border-none focus:ring-0 cursor-pointer uppercase rounded-none h-full"
            >
              {EXAMPLE_NAMES.map((name) => (
                <option key={name} value={name}>{name}</option>
              ))}
            </select>
            <div class="absolute inset-y-0 right-0 pr-3 flex items-center pointer-events-none">
              <svg
                class="w-3 h-3 md:w-4 md:h-4 fill-current"
                viewBox="0 0 20 20"
              >
                <path d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" />
              </svg>
            </div>
          </div>
        </div>
      </Header>

      <Main>
        <Panel
          title={`Rusk Input (${stats.value.sourceLines} lines)`}
          class="w-full md:w-1/2"
          action={
            <button
              type="button"
              onClick={formatInput}
              class="text-xs uppercase font-bold px-3 py-1 border-2 border-black bg-white hover:bg-black hover:text-white"
            >
              Format
            </button>
          }
        >
          <InputEditor
            editorRef={inputRef}
            value={inputCode.value}
            onChange={(value) => inputCode.value = value}
            onScroll={() => handleScroll("input")}
          />
        </Panel>

        <Panel
          title={`${
            outputMode.value === "rust"
              ? "Rust Output"
              : outputMode.value === "syntax-tree"
              ? "Syntax Tree"
              : "Run Output"
          } (${stats.value.outputLines} lines${
            outputMode.value === "run"
              ? ""
              : `, ${formatMs(stats.value.outputMs)}`
          })`}
          class="w-full md:w-1/2"
          action={
            <div class="flex gap-2">
              {isDevServer && (
                <button
                  type="button"
                  onClick={runRust}
                  disabled={runState.value === "running" ||
                    !!transpiled.value.error}
                  class="text-xs uppercase font-bold px-3 py-1 border-2 border-black bg-white hover:bg-black hover:text-white disabled:opacity-50 disabled:hover:bg-white disabled:hover:text-black"
                >
                  {runState.value === "running" ? "Running" : "▶ Run"}
                </button>
              )}
              <button
                type="button"
                onClick={copyOutput}
                class="text-xs uppercase font-bold px-3 py-1 border-2 border-black bg-white hover:bg-black hover:text-white"
              >
                {copied.value ? "Copied" : "Copy"}
              </button>
            </div>
          }
        >
          <OutputDisplay
            outputRef={outputRef}
            value={outputText.value}
            language={outputMode.value === "rust"
              ? "rust"
              : outputMode.value === "syntax-tree"
              ? "json"
              : "text"}
            error={transpiled.value.error}
            onScroll={() => handleScroll("output")}
          />
        </Panel>
      </Main>
    </Layout>
  )
}

function countLines(value: string): number {
  if (!value) return 0
  return value.replace(/\n$/, "").split("\n").length
}

function formatMs(value: number): string {
  return `${value.toFixed(value < 10 ? 2 : 1)} ms`
}

function formatRunOutput(result: RunResponse): string {
  const output = `${result.stdout}${result.stderr}`.trimEnd()
  const status = result.timedOut
    ? `${result.stage} timed out`
    : `${result.stage} exit ${result.status ?? "unknown"}`
  return output ? `${output}\n\n[${status}]` : `[${status}]`
}

function stringifyError(error: unknown): string {
  if (error instanceof Error) return error.message
  return String(error)
}
