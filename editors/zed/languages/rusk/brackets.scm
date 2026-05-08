((punctuation) @open
  (#any-of? @open "(" "[" "{"))

((punctuation) @close
  (#any-of? @close ")" "]" "}"))

(string) @open @close
(char) @open @close
