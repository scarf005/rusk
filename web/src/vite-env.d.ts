type ImportMetaGlobOptions = {
  eager?: boolean
  import?: string
  query?: string | Record<string, string | number | boolean>
}

interface ImportMeta {
  glob<T = unknown>(
    pattern: string | string[],
    options?: ImportMetaGlobOptions,
  ): Record<string, T>
}
