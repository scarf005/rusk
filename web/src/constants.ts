import helloUser from "../../examples/hello_user.rsk?raw"
import moduleLayout from "../../examples/module_layout.rsk?raw"
import resultFlow from "../../examples/result_flow.rsk?raw"

export const TEMPLATES = {
  "Hello User": helloUser,
  "Result Flow": resultFlow,
  "Module Layout": moduleLayout,
} as const

export type TemplateName = keyof typeof TEMPLATES

export const TEMPLATE_NAMES = Object.keys(TEMPLATES) as TemplateName[]
