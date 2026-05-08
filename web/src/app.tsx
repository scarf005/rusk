import { useRef } from "preact/hooks"
import { computed, signal, useSignal } from "@preact/signals"
import { transpile_syntax_tree_json, transpile_to_rust } from "./wasm/rusk.js"
import { InputEditor, OutputDisplay } from "./Editor.tsx"
import { Header, Layout, Main, Panel } from "./Layout.tsx"
import { TEMPLATE_NAMES, TemplateName, TEMPLATES } from "./constants.ts"

type OutputMode = "rust" | "syntax-tree"

const selectedTemplate = signal<TemplateName>("Hello User")
const inputCode = signal<string>(TEMPLATES[selectedTemplate.value])
const outputMode = signal<OutputMode>("rust")

const transpiled = computed(() => {
  try {
    const rust = transpile_to_rust(inputCode.value)
    const syntaxTree = transpile_syntax_tree_json(inputCode.value)
    return { rust, syntaxTree, error: "" }
  } catch (error) {
    return {
      rust: "",
      syntaxTree: "",
      error: stringifyError(error),
    }
  }
})

const outputText = computed(() => {
  if (transpiled.value.error) {
    return "// Fix the Rusk source to see generated output."
  }
  return outputMode.value === "rust"
    ? transpiled.value.rust
    : transpiled.value.syntaxTree
})

const stats = computed(() => ({
  sourceLines: countLines(inputCode.value),
  outputLines: countLines(outputText.value),
}))

export function App() {
  const copied = useSignal(false)
  const inputRef = useRef<HTMLTextAreaElement>(null)
  const outputRef = useRef<HTMLDivElement>(null)
  const isScrollingRef = useRef(false)

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

  const loadTemplate = (name: TemplateName) => {
    selectedTemplate.value = name
    inputCode.value = TEMPLATES[name]
    inputRef.current?.scrollTo({ top: 0 })
    outputRef.current?.scrollTo({ top: 0 })
  }

  const copyOutput = async () => {
    await navigator.clipboard.writeText(outputText.value)
    copied.value = true
    setTimeout(() => copied.value = false, 900)
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
          {["rust", "syntax-tree"].map((mode) => (
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
              {mode === "rust" ? "Rust Output" : "Syntax Tree"}
            </button>
          ))}
        </div>

        <div class="flex-none w-full lg:w-auto flex flex-row border-b-2 lg:border-b-0 border-black bg-white">
          <div class="flex-1 lg:flex-none relative border-r-2 border-black">
            <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
              <span class="text-[10px] md:text-xs font-bold uppercase mr-2 opacity-50">
                TMPL:
              </span>
            </div>
            <select
              value={selectedTemplate.value}
              onChange={(event) =>
                loadTemplate(event.currentTarget.value as TemplateName)}
              class="appearance-none w-full lg:w-[210px] bg-white pl-14 pr-8 py-3 text-xs md:text-sm font-bold border-none focus:ring-0 cursor-pointer uppercase rounded-none h-full"
            >
              {TEMPLATE_NAMES.map((name) => (
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
            outputMode.value === "rust" ? "Rust Output" : "Syntax Tree"
          } (${stats.value.outputLines} lines)`}
          class="w-full md:w-1/2"
          action={
            <button
              type="button"
              onClick={copyOutput}
              class="text-xs uppercase font-bold px-3 py-1 border-2 border-black bg-white hover:bg-black hover:text-white"
            >
              {copied.value ? "Copied" : "Copy"}
            </button>
          }
        >
          <OutputDisplay
            outputRef={outputRef}
            value={outputText.value}
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

function stringifyError(error: unknown): string {
  if (error instanceof Error) return error.message
  return String(error)
}
