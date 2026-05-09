/// <reference path="./vite-env.d.ts" />

type Example = {
  name: string
  slug: string
  source: string
}

const ruskExampleModules = import.meta.glob<string>("../../examples/*.rsk", {
  eager: true,
  import: "default",
  query: "?raw",
})

const rustExampleModules = import.meta.glob<string>("../../examples/*.rs", {
  eager: true,
  import: "default",
  query: "?raw",
})

const stemFromPath = (path: string) =>
  path.match(/\/([^/.]+)\.[^.]+$/)?.[1] ?? path

const slugFromStem = (stem: string) => stem.replaceAll("_", "-")

const nameFromStem = (stem: string) =>
  stem.replaceAll("_", " ").replace(/\b\w/g, (letter) => letter.toUpperCase())

const examplesFromModules = (modules: Record<string, string>) =>
  Object.entries(modules)
    .map(([path, source]) => {
      const stem = stemFromPath(path)
      return { name: nameFromStem(stem), slug: slugFromStem(stem), source }
    })
    .sort((left, right) => left.name.localeCompare(right.name))

export const EXAMPLES = examplesFromModules(ruskExampleModules)
export const RUST_EXAMPLES = examplesFromModules(rustExampleModules)

export type ExampleName = (typeof EXAMPLES)[number]["name"]
export type RustExampleName = (typeof RUST_EXAMPLES)[number]["name"]

const defaultExample = (examples: Example[], slug: string) =>
  examples.find((example) => example.slug === slug) ?? examples[0]

export const DEFAULT_EXAMPLE_NAME: ExampleName = defaultExample(
  EXAMPLES,
  "hello-user",
).name
export const DEFAULT_RUST_EXAMPLE_NAME: RustExampleName = defaultExample(
  RUST_EXAMPLES,
  "rust-to-rusk",
).name
export const EXAMPLE_NAMES = EXAMPLES.map(({ name }) => name) as ExampleName[]
export const RUST_EXAMPLE_NAMES = RUST_EXAMPLES.map(({ name }) =>
  name
) as RustExampleName[]

const EXAMPLES_BY_NAME = Object.fromEntries(
  EXAMPLES.map((example) => [example.name, example]),
) as Record<ExampleName, (typeof EXAMPLES)[number]>

const EXAMPLE_NAMES_BY_SLUG = Object.fromEntries(
  EXAMPLES.map((example) => [example.slug, example.name]),
) as Record<string, ExampleName>

const RUST_EXAMPLES_BY_NAME = Object.fromEntries(
  RUST_EXAMPLES.map((example) => [example.name, example]),
) as Record<RustExampleName, (typeof RUST_EXAMPLES)[number]>

const RUST_EXAMPLE_NAMES_BY_SLUG = Object.fromEntries(
  RUST_EXAMPLES.map((example) => [example.slug, example.name]),
) as Record<string, RustExampleName>

export const exampleSource = (name: ExampleName) =>
  EXAMPLES_BY_NAME[name].source

export const rustExampleSource = (name: RustExampleName) =>
  RUST_EXAMPLES_BY_NAME[name].source

export const examplePath = (name: ExampleName) =>
  `/examples/${EXAMPLES_BY_NAME[name].slug}`

export const rustExamplePath = (name: RustExampleName) =>
  `/rust-examples/${RUST_EXAMPLES_BY_NAME[name].slug}`

export const exampleNameFromSlug = (slug: string) =>
  EXAMPLE_NAMES_BY_SLUG[slug] ?? null

export const rustExampleNameFromSlug = (slug: string) =>
  RUST_EXAMPLE_NAMES_BY_SLUG[slug] ?? null

export const exampleNameFromPath = (path: string) => {
  const slug = path.replace(/^#/, "").split("?")[0].match(
    /^\/examples\/([^/]+)$/,
  )
    ?.[1]
  return slug ? exampleNameFromSlug(slug) : null
}

export const rustExampleNameFromPath = (path: string) => {
  const slug = path.replace(/^#/, "").split("?")[0].match(
    /^\/rust-examples\/([^/]+)$/,
  )
    ?.[1]
  return slug ? rustExampleNameFromSlug(slug) : null
}
