import { Ref } from "preact"
import { useEffect, useRef } from "preact/hooks"
import { EditorState, Extension } from "@codemirror/state"
import {
  EditorView,
  highlightSpecialChars,
  keymap,
  lineNumbers,
} from "@codemirror/view"
import {
  defaultHighlightStyle,
  StreamLanguage,
  syntaxHighlighting,
} from "@codemirror/language"
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands"
import { json } from "@codemirror/lang-json"
import { rust } from "@codemirror/lang-rust"

type CodeLanguage = "rusk" | "rust" | "json"

type RuskState = { inString: boolean }

const rusk = StreamLanguage.define<RuskState>({
  startState: () => ({ inString: false }),
  token: (stream, state) => {
    if (stream.sol()) stream.eatSpace()

    if (state.inString) {
      while (!stream.eol()) {
        const next = stream.next()
        if (next === "\\") stream.next()
        if (next === '"') {
          state.inString = false
          break
        }
      }
      return "string"
    }

    if (stream.eatSpace()) return null
    if (stream.match("//")) {
      stream.skipToEnd()
      return "comment"
    }
    if (stream.match(/^#!?\[[^\]]*\]/)) return "meta"
    if (stream.match('"')) {
      state.inString = true
      return "string"
    }
    if (stream.match(/^\d[\d_]*/)) return "number"
    if (
      stream.match(
        /^(pub|struct|enum|trait|impl|fn|let|mut|if|then|else|match|while|for|loop|use|mod|macro_rules|Self|self|return|break|continue|async|unsafe|do)\b/,
      )
    ) {
      return "keyword"
    }
    if (
      stream.match(
        /^(bool|char|str|String|usize|isize|u8|u16|u32|u64|u128|i8|i16|i32|i64|i128|f32|f64)\b/,
      )
    ) {
      return "typeName"
    }
    if (stream.match(/^[A-Z][A-Za-z0-9_]*/)) return "typeName"
    if (stream.match(/^[a-z_][A-Za-z0-9_]*(?=!)/)) return "variableName.special"
    if (stream.match(/^[+\-*\/%=<>!&|.:]+/)) return "operator"

    stream.next()
    return null
  },
})

const languageExtension = (language: CodeLanguage): Extension => {
  if (language === "rust") return rust()
  if (language === "json") return json()
  return rusk
}

const extensions = ({
  language,
  editable,
  onChange,
}: {
  language: CodeLanguage
  editable: boolean
  onChange?: (value: string) => void
}): Extension[] => [
  lineNumbers(),
  highlightSpecialChars(),
  history(),
  syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
  languageExtension(language),
  EditorState.readOnly.of(!editable),
  EditorView.editable.of(editable),
  EditorView.lineWrapping,
  EditorView.theme({
    "&": {
      height: "100%",
      backgroundColor: editable ? "#ffffff" : "#f9fafb",
      color: "#000000",
      fontSize: "0.875rem",
    },
    ".cm-scroller": {
      fontFamily: "var(--font-mono)",
      lineHeight: "1.625",
    },
    ".cm-content": { padding: "1rem", minHeight: "100%" },
    ".cm-gutters": {
      backgroundColor: editable ? "#ffffff" : "#f9fafb",
      borderRight: "2px solid #000000",
      color: "#6b7280",
    },
    ".cm-focused": { outline: "none" },
    ".cm-content ::selection": {
      backgroundColor: "#000000",
      color: "#ffffff",
    },
  }),
  keymap.of([...defaultKeymap, ...historyKeymap]),
  EditorView.updateListener.of((update) => {
    if (update.docChanged) onChange?.(update.state.doc.toString())
  }),
]

const setRef = <T,>(ref: Ref<T> | undefined, value: T | null) => {
  if (!ref) return
  if (typeof ref === "function") ref(value)
  else ref.current = value
}

interface CodeMirrorProps {
  value: string
  language: CodeLanguage
  editable: boolean
  onChange?: (value: string) => void
  onScroll?: () => void
  scrollRef?: Ref<HTMLElement>
}

const CodeMirror = ({
  value,
  language,
  editable,
  onChange,
  onScroll,
  scrollRef,
}: CodeMirrorProps) => {
  const hostRef = useRef<HTMLDivElement>(null)
  const viewRef = useRef<EditorView | null>(null)
  const changeRef = useRef(onChange)
  const scrollRefCallback = useRef(onScroll)

  changeRef.current = onChange
  scrollRefCallback.current = onScroll

  useEffect(() => {
    if (!hostRef.current) return

    const view = new EditorView({
      parent: hostRef.current,
      state: EditorState.create({
        doc: value,
        extensions: extensions({
          language,
          editable,
          onChange: (next) => changeRef.current?.(next),
        }),
      }),
    })
    const handleScroll = () => scrollRefCallback.current?.()
    view.scrollDOM.addEventListener("scroll", handleScroll)
    viewRef.current = view
    setRef(scrollRef, view.scrollDOM)

    return () => {
      setRef(scrollRef, null)
      view.scrollDOM.removeEventListener("scroll", handleScroll)
      view.destroy()
      viewRef.current = null
    }
  }, [language, editable])

  useEffect(() => {
    const view = viewRef.current
    if (!view) return
    const current = view.state.doc.toString()
    if (current === value) return
    view.dispatch({ changes: { from: 0, to: current.length, insert: value } })
  }, [value])

  return <div ref={hostRef} class="h-full w-full" />
}

interface InputEditorProps {
  value: string
  onChange: (value: string) => void
  onScroll?: () => void
  editorRef?: Ref<HTMLElement>
}

export const InputEditor = ({
  value,
  onChange,
  onScroll,
  editorRef,
}: InputEditorProps) => (
  <CodeMirror
    value={value}
    language="rusk"
    editable
    onChange={onChange}
    onScroll={onScroll}
    scrollRef={editorRef}
  />
)

interface OutputDisplayProps {
  value: string
  language: "rust" | "json"
  error?: string
  onScroll?: () => void
  outputRef?: Ref<HTMLElement>
}

export const OutputDisplay = ({
  value,
  language,
  error,
  onScroll,
  outputRef,
}: OutputDisplayProps) => (
  <div class={`relative w-full h-full ${error ? "bg-red-50" : "bg-gray-50"}`}>
    {error && (
      <div class="absolute inset-x-0 top-0 z-10 border-b-2 border-black bg-black text-white px-4 py-2 text-xs font-bold uppercase">
        Transpile error: {error}
      </div>
    )}
    <CodeMirror
      value={value}
      language={language}
      editable={false}
      onScroll={onScroll}
      scrollRef={outputRef}
    />
  </div>
)
