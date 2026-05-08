import arraySubscription from "../../examples/array_subscription.rsk?raw"
import asyncFunctions from "../../examples/async_functions.rsk?raw"
import closures from "../../examples/closures.rsk?raw"
import complexAmbiguous from "../../examples/complex_ambiguous.rsk?raw"
import constGenerics from "../../examples/const_generics.rsk?raw"
import controlFlow from "../../examples/control_flow.rsk?raw"
import enumMatch from "../../examples/enum_match.rsk?raw"
import generics from "../../examples/generics.rsk?raw"
import helloUser from "../../examples/hello_user.rsk?raw"
import iteratorChaining from "../../examples/iterator_chaining.rsk?raw"
import lifetimes from "../../examples/lifetimes.rsk?raw"
import macros from "../../examples/macros.rsk?raw"
import moduleLayout from "../../examples/module_layout.rsk?raw"
import multilineClosures from "../../examples/multiline_closures.rsk?raw"
import nestedModules from "../../examples/nested_modules.rsk?raw"
import optionResult from "../../examples/option_result.rsk?raw"
import pathsAndAttributes from "../../examples/paths_and_attributes.rsk?raw"
import patternMatchingComplex from "../../examples/pattern_matching_complex.rsk?raw"
import rawStrings from "../../examples/raw_strings.rsk?raw"
import resultFlow from "../../examples/result_flow.rsk?raw"
import rustsUglySyntax from "../../examples/rusts_ugly_syntax.rsk?raw"
import stringParsing from "../../examples/string_parsing.rsk?raw"
import traitsImpl from "../../examples/traits_impl.rsk?raw"
import tupleDestructuring from "../../examples/tuple_destructuring.rsk?raw"
import unsafeBlock from "../../examples/unsafe_block.rsk?raw"
import whereBounds from "../../examples/where_bounds.rsk?raw"

export const EXAMPLES = [
  { name: "Hello User", slug: "hello-user", source: helloUser },
  { name: "Generics", slug: "generics", source: generics },
  { name: "Lifetimes", slug: "lifetimes", source: lifetimes },
  { name: "Const Generics", slug: "const-generics", source: constGenerics },
  {
    name: "Array Subscription",
    slug: "array-subscription",
    source: arraySubscription,
  },
  {
    name: "Complex Ambiguous",
    slug: "complex-ambiguous",
    source: complexAmbiguous,
  },
  {
    name: "Pattern Matching",
    slug: "pattern-matching",
    source: patternMatchingComplex,
  },
  {
    name: "Iterator Chaining",
    slug: "iterator-chaining",
    source: iteratorChaining,
  },
  { name: "Macros", slug: "macros", source: macros },
  { name: "Option Result", slug: "option-result", source: optionResult },
  {
    name: "Tuple Destructuring",
    slug: "tuple-destructuring",
    source: tupleDestructuring,
  },
  { name: "String Parsing", slug: "string-parsing", source: stringParsing },
  { name: "Closures", slug: "closures", source: closures },
  {
    name: "Multiline Closures",
    slug: "multiline-closures",
    source: multilineClosures,
  },
  { name: "Async Functions", slug: "async-functions", source: asyncFunctions },
  { name: "Unsafe Block", slug: "unsafe-block", source: unsafeBlock },
  { name: "Nested Modules", slug: "nested-modules", source: nestedModules },
  { name: "Where Bounds", slug: "where-bounds", source: whereBounds },
  { name: "Enum Match", slug: "enum-match", source: enumMatch },
  { name: "Traits Impl", slug: "traits-impl", source: traitsImpl },
  { name: "Control Flow", slug: "control-flow", source: controlFlow },
  {
    name: "Paths Attributes",
    slug: "paths-attributes",
    source: pathsAndAttributes,
  },
  { name: "Raw Strings", slug: "raw-strings", source: rawStrings },
  { name: "Result Flow", slug: "result-flow", source: resultFlow },
  {
    name: "Rust's Ugly Syntax",
    slug: "rusts-ugly-syntax",
    source: rustsUglySyntax,
  },
  { name: "Module Layout", slug: "module-layout", source: moduleLayout },
] as const

export type ExampleName = (typeof EXAMPLES)[number]["name"]

export const DEFAULT_EXAMPLE_NAME: ExampleName = "Hello User"
export const EXAMPLE_NAMES = EXAMPLES.map(({ name }) => name) as ExampleName[]

const EXAMPLES_BY_NAME = Object.fromEntries(
  EXAMPLES.map((example) => [example.name, example]),
) as Record<ExampleName, (typeof EXAMPLES)[number]>

const EXAMPLE_NAMES_BY_SLUG = Object.fromEntries(
  EXAMPLES.map((example) => [example.slug, example.name]),
) as Record<string, ExampleName>

export const exampleSource = (name: ExampleName) =>
  EXAMPLES_BY_NAME[name].source

export const examplePath = (name: ExampleName) =>
  `/examples/${EXAMPLES_BY_NAME[name].slug}`

export const exampleNameFromSlug = (slug: string) =>
  EXAMPLE_NAMES_BY_SLUG[slug] ?? null

export const exampleNameFromPath = (path: string) => {
  const slug = path.replace(/^#/, "").split("?")[0].match(
    /^\/examples\/([^/]+)$/,
  )
    ?.[1]
  return slug ? exampleNameFromSlug(slug) : null
}
