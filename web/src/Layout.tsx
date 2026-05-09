import { ComponentChildren } from "preact"

export const Layout = ({ children }: { children: ComponentChildren }) => (
  <div class="fixed inset-0 flex flex-col bg-white text-black font-mono overflow-hidden">
    {children}
  </div>
)

export const Header = ({ children }: { children: ComponentChildren }) => (
  <header class="flex-none border-b-2 border-black w-full bg-white z-10 flex flex-col lg:flex-row">
    {children}
  </header>
)

export const Main = ({ children }: { children: ComponentChildren }) => (
  <main class="flex-1 flex flex-col md:flex-row min-h-0 overflow-hidden md:overflow-visible">
    {children}
  </main>
)

export const Panel = ({
  title,
  action,
  children,
  class: className = "",
  border =
    "border-b-2 md:border-b-0 md:border-r-2 border-black last:border-r-0 last:border-b-0",
}: {
  title: string
  action?: ComponentChildren
  children: ComponentChildren
  class?: string
  border?: string
}) => (
  <section
    class={`flex flex-col ${border} ${className}`}
  >
    <div class="flex-none min-h-[50px] py-2 px-4 border-b-2 border-black bg-white uppercase font-bold text-sm tracking-wider flex items-center justify-between gap-3">
      <span>{title}</span>
      {action}
    </div>
    <div class="flex-1 relative min-h-0 bg-white">
      {children}
    </div>
  </section>
)
