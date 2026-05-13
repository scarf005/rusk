import { useEffect, useRef } from "preact/hooks"
import { computed, signal, useSignal } from "@preact/signals"
import { useLocation } from "wouter-preact"
import {
  convert_ruk_to_rusk,
  convert_ruk_to_rust,
  convert_rusk_to_ruk,
  convert_rust_to_ruk,
  convert_rust_to_rusk,
  format_rusk,
  transpile_syntax_tree_json,
  transpile_to_rust,
} from "./wasm/rusk.js"
import { InputEditor, OutputDisplay } from "./Editor.tsx"
import { Header, Layout, Main, Panel } from "./Layout.tsx"
import {
  DEFAULT_EXAMPLE_NAME,
  DEFAULT_RUK_EXAMPLE_NAME,
  EXAMPLE_NAMES,
  ExampleName,
  exampleNameFromPath,
  examplePath,
  exampleSource,
  RUK_EXAMPLE_NAMES,
  RukExampleName,
  rukExampleNameFromPath,
  rukExamplePath,
  rukExampleSource,
} from "./constants.ts"

type SourcePanel = "rusk" | "ruk"
type EditSide = "source" | "rust"
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

const sourcePanel = signal<SourcePanel>("rusk")
const editSide = signal<EditSide>("source")
const selectedExample = signal<ExampleName>(DEFAULT_EXAMPLE_NAME)
const selectedRukExample = signal<RukExampleName>(DEFAULT_RUK_EXAMPLE_NAME)
const sourceCode = signal<string>(exampleSource(DEFAULT_EXAMPLE_NAME))
const rustCode = signal<string>(transpile_to_rust(sourceCode.value))
const outputMode = signal<OutputMode>("rust")
const runOutput = signal("")
const runResult = signal<RunResponse | null>(null)
const runState = signal<RunState>("idle")

const converted = computed(() => {
  const started = performance.now()
  try {
    const rustStarted = performance.now()
    const rust = editSide.value === "rust"
      ? rustCode.value
      : sourcePanel.value === "rusk"
      ? transpile_to_rust(sourceCode.value)
      : convert_ruk_to_rust(sourceCode.value)
    const rustMs = performance.now() - rustStarted

    const ruskStarted = performance.now()
    const rusk = editSide.value === "rust"
      ? convert_rust_to_rusk(rust)
      : sourcePanel.value === "rusk"
      ? sourceCode.value
      : convert_ruk_to_rusk(sourceCode.value)
    const ruskMs = performance.now() - ruskStarted

    const rukStarted = performance.now()
    const ruk = editSide.value === "rust"
      ? convert_rust_to_ruk(rust)
      : sourcePanel.value === "ruk"
      ? sourceCode.value
      : convert_rusk_to_ruk(sourceCode.value)
    const rukMs = performance.now() - rukStarted

    const syntaxTreeStarted = performance.now()
    const syntaxTree = transpile_syntax_tree_json(rusk)
    const syntaxTreeMs = performance.now() - syntaxTreeStarted
    return {
      rust,
      rusk,
      ruk,
      syntaxTree,
      rustMs,
      ruskMs,
      rukMs,
      syntaxTreeMs,
      error: "",
    }
  } catch (error) {
    const elapsedMs = performance.now() - started
    return {
      rust: "",
      rusk: "",
      ruk: "",
      syntaxTree: "",
      rustMs: elapsedMs,
      ruskMs: elapsedMs,
      rukMs: elapsedMs,
      syntaxTreeMs: elapsedMs,
      error: stringifyError(error),
    }
  }
})

const sourcePanelText = computed(() => sourceCode.value)
const rustPanelText = computed(() =>
  editSide.value === "rust" ? rustCode.value : converted.value.rust
)
const outputText = computed(() =>
  outputMode.value === "rust" ? rustPanelText.value : converted.value.syntaxTree
)

const stats = computed(() => ({
  sourceLines: countLines(sourcePanelText.value),
  outputLines: countLines(outputText.value),
  outputMs: outputMode.value === "rust"
    ? converted.value.rustMs
    : converted.value.syntaxTreeMs,
}))

export function App() {
  const [location, navigate] = useLocation()
  const copied = useSignal(false)
  const sourceRef = useRef<HTMLElement>(null)
  const outputRef = useRef<HTMLElement>(null)
  const isScrollingRef = useRef(false)

  const clearRun = () => {
    runOutput.value = ""
    runResult.value = null
  }

  const setSourceCode = (value: string) => {
    editSide.value = "source"
    sourceCode.value = value
    clearRun()
  }

  const setRustCode = (value: string) => {
    editSide.value = "rust"
    rustCode.value = value
    try {
      sourceCode.value = sourcePanel.value === "rusk"
        ? convert_rust_to_rusk(value)
        : convert_rust_to_ruk(value)
    } catch (_) {
      // Keep the last source panel text while the Rust panel is invalid.
    }
    clearRun()
  }

  const showExample = (name: ExampleName) => {
    sourcePanel.value = "rusk"
    editSide.value = "source"
    selectedExample.value = name
    sourceCode.value = exampleSource(name)
    rustCode.value = transpile_to_rust(sourceCode.value)
    clearRun()
    sourceRef.current?.scrollTo({ top: 0 })
    outputRef.current?.scrollTo({ top: 0 })
  }

  const showRukExample = (name: RukExampleName) => {
    sourcePanel.value = "ruk"
    editSide.value = "source"
    selectedRukExample.value = name
    sourceCode.value = rukExampleSource(name)
    rustCode.value = convert_ruk_to_rust(sourceCode.value)
    clearRun()
    sourceRef.current?.scrollTo({ top: 0 })
    outputRef.current?.scrollTo({ top: 0 })
  }

  useEffect(() => {
    const path = location === "/" ? globalThis.location.pathname : location
    const ruskName = path === "/"
      ? DEFAULT_EXAMPLE_NAME
      : exampleNameFromPath(path)
    const rukName = rukExampleNameFromPath(path)

    if (path === "/") {
      navigate(examplePath(DEFAULT_EXAMPLE_NAME))
      return
    }

    if (ruskName) {
      if (
        sourcePanel.value === "rusk" &&
        selectedExample.value === ruskName &&
        sourceCode.value === exampleSource(ruskName)
      ) return
      showExample(ruskName)
      return
    }

    if (rukName) {
      if (
        sourcePanel.value === "ruk" &&
        selectedRukExample.value === rukName &&
        sourceCode.value === rukExampleSource(rukName)
      ) return
      showRukExample(rukName)
      return
    }

    navigate(examplePath(DEFAULT_EXAMPLE_NAME))
  }, [location, navigate])

  const handleScroll = (source: "source" | "output") => {
    if (isScrollingRef.current) return

    const sourceEl = sourceRef.current
    const outputEl = outputRef.current
    if (!sourceEl || !outputEl) return

    isScrollingRef.current = true
    if (source === "source") {
      const ratio = sourceEl.scrollTop /
        (sourceEl.scrollHeight - sourceEl.clientHeight || 1)
      outputEl.scrollTop = ratio *
        (outputEl.scrollHeight - outputEl.clientHeight)
    } else {
      const ratio = outputEl.scrollTop /
        (outputEl.scrollHeight - outputEl.clientHeight || 1)
      sourceEl.scrollTop = ratio *
        (sourceEl.scrollHeight - sourceEl.clientHeight)
    }
    setTimeout(() => isScrollingRef.current = false, 10)
  }

  const loadExample = (name: ExampleName) => {
    showExample(name)
    const path = examplePath(name)
    if (location !== path) navigate(path)
  }

  const loadRukExample = (name: RukExampleName) => {
    showRukExample(name)
    const path = rukExamplePath(name)
    if (location !== path) navigate(path)
  }

  const setSourcePanel = (panel: SourcePanel) => {
    if (panel === sourcePanel.value) return
    const nextSource = panel === "rusk"
      ? converted.value.rusk
      : converted.value.ruk
    const matchingRuskName = EXAMPLE_NAMES.find((name) =>
      name === selectedRukExample.value as string
    )
    const matchingRukName = RUK_EXAMPLE_NAMES.find((name) =>
      name === selectedExample.value as string
    )
    if (panel === "rusk" && matchingRuskName) {
      selectedExample.value = matchingRuskName
    }
    if (panel === "ruk" && matchingRukName) {
      selectedRukExample.value = matchingRukName
    }
    sourcePanel.value = panel
    sourceCode.value = nextSource
    if (panel === "rusk") navigate(examplePath(selectedExample.value))
    else navigate(rukExamplePath(selectedRukExample.value))
  }

  const copyOutput = async () => {
    await navigator.clipboard.writeText(outputText.value)
    copied.value = true
    setTimeout(() => copied.value = false, 900)
  }

  const formatInput = () => {
    if (sourcePanel.value === "rusk") {
      setSourceCode(format_rusk(sourceCode.value, 100))
    }
  }

  const runRust = async () => {
    if (converted.value.error || runState.value === "running") return

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
      if ("error" in result) runOutput.value = result.error
      else {
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
          {(["rusk", "ruk"] as SourcePanel[]).map((panel) => (
            <button
              type="button"
              key={panel}
              onClick={() => setSourcePanel(panel)}
              class={`flex-none px-4 md:px-6 py-3 font-bold text-xs md:text-sm uppercase transition-none border-r-2 border-black focus:outline-none whitespace-nowrap ${
                sourcePanel.value === panel
                  ? "bg-black text-white"
                  : "bg-white text-black hover:bg-gray-200"
              }`}
            >
              {panel.toUpperCase()}
            </button>
          ))}
          {(["rust", "syntax-tree"] as OutputMode[]).map((mode) => (
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
              {mode === "rust" ? "Rust" : "Syntax Tree"}
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
              value={sourcePanel.value === "rusk"
                ? selectedExample.value
                : selectedRukExample.value}
              onChange={(event) =>
                sourcePanel.value === "rusk"
                  ? loadExample(event.currentTarget.value as ExampleName)
                  : loadRukExample(event.currentTarget.value as RukExampleName)}
              class="appearance-none w-full lg:w-[210px] bg-white pl-14 pr-8 py-3 text-xs md:text-sm font-bold border-none focus:ring-0 cursor-pointer uppercase rounded-none h-full"
            >
              {(sourcePanel.value === "rusk"
                ? EXAMPLE_NAMES
                : RUK_EXAMPLE_NAMES).map((name) => (
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
          title={`${sourcePanel.value.toUpperCase()} (${stats.value.sourceLines} lines)`}
          class="w-full md:w-1/2 h-1/2 md:h-full"
          action={sourcePanel.value === "rusk" && (
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
            editorRef={sourceRef}
            value={sourcePanelText.value}
            language={sourcePanel.value}
            onChange={setSourceCode}
            onScroll={() => handleScroll("source")}
          />
        </Panel>

        <div class="w-full md:w-1/2 h-1/2 md:h-full flex flex-col min-h-0">
          <Panel
            title={`${
              outputMode.value === "rust" ? "Rust" : "Syntax Tree"
            } (${stats.value.outputLines} lines, ${
              formatMs(stats.value.outputMs)
            })`}
            class="w-full h-1/2"
            border="border-b-2 border-black"
            action={
              <div class="flex gap-2">
                {isDevServer && outputMode.value === "rust" && (
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
            {outputMode.value === "rust"
              ? (
                <InputEditor
                  editorRef={outputRef}
                  value={rustPanelText.value}
                  language="rust"
                  onChange={setRustCode}
                  onScroll={() => handleScroll("output")}
                />
              )
              : (
                <OutputDisplay
                  outputRef={outputRef}
                  value={outputText.value}
                  language="json"
                  error={converted.value.error}
                  onScroll={() => handleScroll("output")}
                />
              )}
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
