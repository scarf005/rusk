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
}: {
  title: string
  action?: ComponentChildren
  children: ComponentChildren
  class?: string
}) => (
  <section
    class={`flex flex-col h-1/2 md:h-full border-b-2 md:border-b-0 md:border-r-2 border-black last:border-r-0 last:border-b-0 ${className}`}
  >
    <div class="flex-none py-2 px-4 border-b-2 border-black bg-white uppercase font-bold text-sm tracking-wider flex items-center justify-between gap-3">
      <span>{title}</span>
      {action}
    </div>
    <div class="flex-1 relative min-h-0 bg-white">
      {children}
    </div>
  </section>
)

export const Banner = ({ children }: { children: ComponentChildren }) => (
  <div class="absolute inset-x-4 bottom-4 border-2 border-black bg-white p-4 shadow-[6px_6px_0_0_rgba(0,0,0,1)] text-sm font-bold">
    {children}
  </div>
)
