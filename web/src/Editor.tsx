import { Ref } from "preact"

interface InputEditorProps {
  value: string
  onChange: (value: string) => void
  onScroll?: () => void
  editorRef?: Ref<HTMLTextAreaElement>
}

export const InputEditor = ({
  value,
  onChange,
  onScroll,
  editorRef,
}: InputEditorProps) => (
  <textarea
    ref={editorRef}
    class="w-full h-full p-4 resize-none focus:outline-none focus:ring-0 text-sm leading-relaxed font-mono bg-white text-black rounded-none border-0"
    value={value}
    onInput={(event) => onChange(event.currentTarget.value)}
    onScroll={onScroll}
    spellcheck={false}
    placeholder="// Type Rusk source here..."
  />
)

interface OutputDisplayProps {
  value: string
  error?: string
  onScroll?: () => void
  outputRef?: Ref<HTMLDivElement>
}

export const OutputDisplay = ({
  value,
  error,
  onScroll,
  outputRef,
}: OutputDisplayProps) => (
  <div
    ref={outputRef}
    onScroll={onScroll}
    class={`relative w-full h-full overflow-auto ${
      error ? "bg-red-50" : "bg-gray-50"
    }`}
  >
    {error && (
      <div class="sticky top-0 z-10 border-b-2 border-black bg-black text-white px-4 py-2 text-xs font-bold uppercase">
        Transpile error: {error}
      </div>
    )}
    <pre class="p-4 text-sm leading-relaxed font-mono whitespace-pre text-black">
{value}
    </pre>
  </div>
)
