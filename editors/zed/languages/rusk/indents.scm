((keyword) @indent.begin
  (#any-of? @indent.begin "struct" "enum" "trait" "impl" "mod" "fn" "if" "else" "match" "while" "for" "loop" "unsafe" "async"))

((punctuation) @indent.begin
  (#any-of? @indent.begin "(" "[" "{"))

((punctuation) @indent.end
  (#any-of? @indent.end ")" "]" "}"))

(line_comment) @indent.ignore
