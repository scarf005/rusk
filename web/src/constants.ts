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
import nestedModules from "../../examples/nested_modules.rsk?raw"
import optionResult from "../../examples/option_result.rsk?raw"
import pathsAndAttributes from "../../examples/paths_and_attributes.rsk?raw"
import patternMatchingComplex from "../../examples/pattern_matching_complex.rsk?raw"
import rawStrings from "../../examples/raw_strings.rsk?raw"
import resultFlow from "../../examples/result_flow.rsk?raw"
import stringParsing from "../../examples/string_parsing.rsk?raw"
import traitsImpl from "../../examples/traits_impl.rsk?raw"
import tupleDestructuring from "../../examples/tuple_destructuring.rsk?raw"
import unsafeBlock from "../../examples/unsafe_block.rsk?raw"
import whereBounds from "../../examples/where_bounds.rsk?raw"

export const TEMPLATES = {
  "Hello User": helloUser,
  Generics: generics,
  Lifetimes: lifetimes,
  "Const Generics": constGenerics,
  "Array Subscription": arraySubscription,
  "Complex Ambiguous": complexAmbiguous,
  "Pattern Matching": patternMatchingComplex,
  "Iterator Chaining": iteratorChaining,
  Macros: macros,
  "Option Result": optionResult,
  "Tuple Destructuring": tupleDestructuring,
  "String Parsing": stringParsing,
  Closures: closures,
  "Async Functions": asyncFunctions,
  "Unsafe Block": unsafeBlock,
  "Nested Modules": nestedModules,
  "Where Bounds": whereBounds,
  "Enum Match": enumMatch,
  "Traits Impl": traitsImpl,
  "Control Flow": controlFlow,
  "Paths Attributes": pathsAndAttributes,
  "Raw Strings": rawStrings,
  "Result Flow": resultFlow,
  "Module Layout": moduleLayout,
} as const

export type TemplateName = keyof typeof TEMPLATES

export const TEMPLATE_NAMES = Object.keys(TEMPLATES) as TemplateName[]
