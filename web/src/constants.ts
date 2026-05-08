import arraySubscription from "../../examples/array_subscription.rsk?raw"
import complexAmbiguous from "../../examples/complex_ambiguous.rsk?raw"
import controlFlow from "../../examples/control_flow.rsk?raw"
import enumMatch from "../../examples/enum_match.rsk?raw"
import generics from "../../examples/generics.rsk?raw"
import helloUser from "../../examples/hello_user.rsk?raw"
import moduleLayout from "../../examples/module_layout.rsk?raw"
import pathsAndAttributes from "../../examples/paths_and_attributes.rsk?raw"
import resultFlow from "../../examples/result_flow.rsk?raw"
import traitsImpl from "../../examples/traits_impl.rsk?raw"

export const TEMPLATES = {
  "Hello User": helloUser,
  Generics: generics,
  "Array Subscription": arraySubscription,
  "Complex Ambiguous": complexAmbiguous,
  "Enum Match": enumMatch,
  "Traits Impl": traitsImpl,
  "Control Flow": controlFlow,
  "Paths Attributes": pathsAndAttributes,
  "Result Flow": resultFlow,
  "Module Layout": moduleLayout,
} as const

export type TemplateName = keyof typeof TEMPLATES

export const TEMPLATE_NAMES = Object.keys(TEMPLATES) as TemplateName[]
