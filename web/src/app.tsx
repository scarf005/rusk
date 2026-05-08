import { useEffect, useRef } from "preact/hooks"
import { computed, signal, useSignal } from "@preact/signals"
import { useLocation } from "wouter-preact"
import {
  convert_rust_to_rusk,
  format_rusk,
  transpile_syntax_tree_json,
  transpile_to_rust,
} from "./wasm/rusk.js"
import { InputEditor, OutputDisplay } from "./Editor.tsx"
import { Header, Layout, Main, Panel } from "./Layout.tsx"
import {
  DEFAULT_EXAMPLE_NAME,
  DEFAULT_RUST_EXAMPLE_NAME,
  EXAMPLE_NAMES,
  ExampleName,
  exampleNameFromPath,
  examplePath,
  exampleSource,
  RUST_EXAMPLE_NAMES,
  RustExampleName,
  rustExampleNameFromPath,
  rustExamplePath,
  rustExampleSource,
} from "./constants.ts"

type ConvertMode = "rusk-to-rust" | "rust-to-rusk"
type OutputMode = "rust" | "syntax-tree"
type RunState = "idle" | "running"

type RunResponse = {
  ok: boolean
  stage: "compile" | "run"
  status: number | null
  stdout: string
  stderr: string
  timedOut: boolean
  compileMs: number
  runMs: number
  totalMs: number
}

const isDevServer =
  (import.meta as ImportMeta & { env: { DEV: boolean } }).env.DEV

const convertMode = signal<ConvertMode>("rusk-to-rust")
const selectedExample = signal<ExampleName>(DEFAULT_EXAMPLE_NAME)
const selectedRustExample = signal<RustExampleName>(DEFAULT_RUST_EXAMPLE_NAME)
const inputCode = signal<string>(exampleSource(DEFAULT_EXAMPLE_NAME))
const outputMode = signal<OutputMode>("rust")
const runOutput = signal("")
const runResult = signal<RunResponse | null>(null)
const runState = signal<RunState>("idle")

const converted = computed(() => {
  const started = performance.now()
  try {
    if (convertMode.value === "rust-to-rusk") {
      const rusk = convert_rust_to_rusk(inputCode.value)
      const elapsedMs = performance.now() - started
      return {
        rust: "",
        rusk,
        syntaxTree: "",
        rustMs: elapsedMs,
        ruskMs: elapsedMs,
        syntaxTreeMs: 0,
        error: "",
      }
    }

    const rustStarted = performance.now()
    const rust = transpile_to_rust(inputCode.value)
    const rustMs = performance.now() - rustStarted
    const syntaxTreeStarted = performance.now()
    const syntaxTree = transpile_syntax_tree_json(inputCode.value)
    const syntaxTreeMs = performance.now() - syntaxTreeStarted
    return {
      rust,
      rusk: "",
      syntaxTree,
      rustMs,
      ruskMs: 0,
      syntaxTreeMs,
      error: "",
    }
  } catch (error) {
    const elapsedMs = performance.now() - started
    return {
      rust: "",
      rusk: "",
      syntaxTree: "",
      rustMs: elapsedMs,
      ruskMs: elapsedMs,
      syntaxTreeMs: elapsedMs,
      error: stringifyError(error),
    }
  }
})

const outputText = computed(() => {
  if (converted.value.error) {
    return convertMode.value === "rusk-to-rust"
      ? "// Fix the Rusk source to see generated output."
      : "// Fix the Rust source to see generated output."
  }
  if (convertMode.value === "rust-to-rusk") return converted.value.rusk
  return outputMode.value === "rust"
    ? converted.value.rust
    : converted.value.syntaxTree
})

const stats = computed(() => ({
  sourceLines: countLines(inputCode.value),
  outputLines: countLines(outputText.value),
  outputMs: convertMode.value === "rust-to-rusk"
    ? converted.value.ruskMs
    : outputMode.value === "rust"
    ? converted.value.rustMs
    : converted.value.syntaxTreeMs,
}))

export function App() {
  const [location, navigate] = useLocation()
  const copied = useSignal(false)
  const inputRef = useRef<HTMLElement>(null)
  const outputRef = useRef<HTMLElement>(null)
  const isScrollingRef = useRef(false)

  const clearRun = () => {
    runOutput.value = ""
    runResult.value = null
  }

  const setInputCode = (value: string) => {
    inputCode.value = value
    clearRun()
  }

  const showExample = (name: ExampleName) => {
    convertMode.value = "rusk-to-rust"
    selectedExample.value = name
    setInputCode(exampleSource(name))
    inputRef.current?.scrollTo({ top: 0 })
    outputRef.current?.scrollTo({ top: 0 })
  }

  const showRustExample = (name: RustExampleName) => {
    convertMode.value = "rust-to-rusk"
    outputMode.value = "rust"
    selectedRustExample.value = name
    setInputCode(rustExampleSource(name))
    inputRef.current?.scrollTo({ top: 0 })
    outputRef.current?.scrollTo({ top: 0 })
  }

  useEffect(() => {
    const path = location === "/" ? globalThis.location.pathname : location
    const ruskName = path === "/"
      ? DEFAULT_EXAMPLE_NAME
      : exampleNameFromPath(path)
    const rustName = rustExampleNameFromPath(path)

    if (path === "/") {
      navigate(examplePath(DEFAULT_EXAMPLE_NAME))
      return
    }

    if (ruskName) {
      if (
        convertMode.value === "rusk-to-rust" &&
        selectedExample.value === ruskName &&
        inputCode.value === exampleSource(ruskName)
      ) return

      showExample(ruskName)
      return
    }

    if (rustName) {
      if (
        convertMode.value === "rust-to-rusk" &&
        selectedRustExample.value === rustName &&
        inputCode.value === rustExampleSource(rustName)
      ) return

      showRustExample(rustName)
      return
    }

    navigate(examplePath(DEFAULT_EXAMPLE_NAME))
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

  const loadRustExample = (name: RustExampleName) => {
    showRustExample(name)
    const path = rustExamplePath(name)
    if (location === path) return
    navigate(path)
  }

  const setConvertMode = (mode: ConvertMode) => {
    if (mode === "rusk-to-rust") loadExample(selectedExample.value)
    else loadRustExample(selectedRustExample.value)
  }

  const copyOutput = async () => {
    await navigator.clipboard.writeText(outputText.value)
    copied.value = true
    setTimeout(() => copied.value = false, 900)
  }

  const formatInput = () => {
    if (convertMode.value === "rusk-to-rust") {
      setInputCode(format_rusk(inputCode.value, 100))
    }
  }

  const runRust = async () => {
    if (
      convertMode.value !== "rusk-to-rust" ||
      converted.value.error ||
      runState.value === "running"
    ) return

    runState.value = "running"
    runResult.value = null
    runOutput.value = "Running..."
    try {
      const response = await fetch("/api/run", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ rust: converted.value.rust }),
      })
      const result = await response.json() as RunResponse | { error: string }
      if ("error" in result) {
        runOutput.value = result.error
      } else {
        runResult.value = result
        runOutput.value = formatRunOutput(result)
      }
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
          {(["rusk-to-rust", "rust-to-rusk"] as ConvertMode[]).map((mode) => (
            <button
              type="button"
              key={mode}
              onClick={() => setConvertMode(mode)}
              class={`flex-none px-4 md:px-6 py-3 font-bold text-xs md:text-sm uppercase transition-none border-r-2 border-black focus:outline-none whitespace-nowrap ${
                convertMode.value === mode
                  ? "bg-black text-white"
                  : "bg-white text-black hover:bg-gray-200"
              }`}
            >
              {mode === "rusk-to-rust" ? "Rusk → Rust" : "Rust → Rusk"}
            </button>
          ))}
          {convertMode.value === "rusk-to-rust" &&
            (["rust", "syntax-tree"] as OutputMode[]).map((mode) => (
              <button
                type="button"
                key={mode}
                onClick={() => outputMode.value = mode}
                class={`flex-none px-4 md:px-6 py-3 font-bold text-xs md:text-sm uppercase transition-none border-r-2 border-black focus:outline-none whitespace-nowrap ${
                  outputMode.value === mode
                    ? "bg-black text-white"
                    : "bg-white text-black hover:bg-gray-200"
                }`}
              >
                {mode === "rust" ? "Rust Output" : "Syntax Tree"}
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
              value={convertMode.value === "rusk-to-rust"
                ? selectedExample.value
                : selectedRustExample.value}
              onChange={(event) =>
                convertMode.value === "rusk-to-rust"
                  ? loadExample(event.currentTarget.value as ExampleName)
                  : loadRustExample(
                    event.currentTarget.value as RustExampleName,
                  )}
              class="appearance-none w-full lg:w-[210px] bg-white pl-14 pr-8 py-3 text-xs md:text-sm font-bold border-none focus:ring-0 cursor-pointer uppercase rounded-none h-full"
            >
              {(convertMode.value === "rusk-to-rust"
                ? EXAMPLE_NAMES
                : RUST_EXAMPLE_NAMES).map((name) => (
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
          title={`${
            convertMode.value === "rusk-to-rust" ? "Rusk" : "Rust"
          } Input (${stats.value.sourceLines} lines)`}
          class="w-full md:w-1/2 h-1/2 md:h-full"
          action={convertMode.value === "rusk-to-rust" && (
            <button
              type="button"
              onClick={formatInput}
              class="text-xs uppercase font-bold px-3 py-1 border-2 border-black bg-white hover:bg-black hover:text-white"
            >
              Format
            </button>
          )}
        >
          <InputEditor
            editorRef={inputRef}
            value={inputCode.value}
            language={convertMode.value === "rusk-to-rust" ? "rusk" : "rust"}
            onChange={setInputCode}
            onScroll={() => handleScroll("input")}
          />
        </Panel>

        <div class="w-full md:w-1/2 h-1/2 md:h-full flex flex-col min-h-0">
          <Panel
            title={`${
              convertMode.value === "rust-to-rusk"
                ? "Rusk Output"
                : outputMode.value === "rust"
                ? "Rust Output"
                : "Syntax Tree"
            } (${stats.value.outputLines} lines, ${
              formatMs(stats.value.outputMs)
            })`}
            class="w-full h-1/2"
            border="border-b-2 border-black"
            action={
              <div class="flex gap-2">
                {isDevServer && convertMode.value === "rusk-to-rust" && (
                  <button
                    type="button"
                    onClick={runRust}
                    disabled={runState.value === "running" ||
                      !!converted.value.error}
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
              language={convertMode.value === "rust-to-rusk"
                ? "rusk"
                : outputMode.value === "rust"
                ? "rust"
                : "json"}
              error={converted.value.error}
              onScroll={() => handleScroll("output")}
            />
          </Panel>

          <Panel
            title={`Run Output (${countLines(runOutput.value)} lines${
              runResult.value ? `, ${formatMs(runResult.value.totalMs)}` : ""
            })`}
            class="w-full h-1/2"
            border="border-b-0 border-black"
          >
            <OutputDisplay value={runOutput.value} language="text" />
          </Panel>
        </div>
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
  const timing = `compile ${formatMs(result.compileMs)}, run ${
    formatMs(result.runMs)
  }, total ${formatMs(result.totalMs)}`
  return output
    ? `${output}\n\n[${status}, ${timing}]`
    : `[${status}, ${timing}]`
}

function stringifyError(error: unknown): string {
  if (error instanceof Error) return error.message
  return String(error)
}
