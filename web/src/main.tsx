import "./index.css"
import { render } from "preact"
import { Router } from "wouter-preact"
import { useHashLocation } from "wouter-preact/use-hash-location"
import { App } from "./app.tsx"

render(
  <Router hook={useHashLocation}>
    <App />
  </Router>,
  document.getElementById("app") as HTMLElement,
)
