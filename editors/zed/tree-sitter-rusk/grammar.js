module.exports = grammar({
  name: "rusk",

  extras: ($) => [/\s/, $.line_comment],

  word: ($) => $.identifier,

  rules: {
    source_file: ($) => repeat($._token),

    _token: ($) => choice(
      $.attribute,
      $.raw_string,
      $.string,
      $.char,
      $.number,
      $.keyword,
      $.builtin_type,
      $.macro_identifier,
      $.type_identifier,
      $.identifier,
      $.operator,
      $.punctuation,
    ),

    line_comment: (_) => token(seq("//", /.*/)),

    attribute: (_) => token(/#!?\[[^\]\n]*\]/),

    raw_string: (_) => token(/r#*"([^"\n]|"[^#])*"#*/),

    string: (_) => token(seq(
      '"',
      repeat(choice(/[^"\\\n]+/, /\\./)),
      '"',
    )),

    char: (_) => token(seq(
      "'",
      choice(/[^'\\\n]/, /\\./),
      "'",
    )),

    number: (_) => token(/\d[\d_]*(\.\d[\d_]*)?/),

    keyword: (_) => token(choice(
      "as",
      "async",
      "await",
      "break",
      "const",
      "continue",
      "crate",
      "else",
      "enum",
      "extern",
      "false",
      "fn",
      "for",
      "if",
      "impl",
      "in",
      "let",
      "loop",
      "match",
      "mod",
      "move",
      "mut",
      "pub",
      "ref",
      "return",
      "Self",
      "self",
      "static",
      "struct",
      "then",
      "trait",
      "true",
      "type",
      "unsafe",
      "use",
      "where",
      "while",
    )),

    builtin_type: (_) => token(choice(
      "bool",
      "char",
      "f32",
      "f64",
      "i8",
      "i16",
      "i32",
      "i64",
      "i128",
      "isize",
      "str",
      "u8",
      "u16",
      "u32",
      "u64",
      "u128",
      "usize",
    )),

    macro_identifier: (_) => token(/[A-Za-z_][A-Za-z0-9_]*!/),

    type_identifier: (_) => token(/[A-Z][A-Za-z0-9_]*/),

    identifier: (_) => token(/[a-z_][A-Za-z0-9_]*/),

    operator: (_) => token(choice(
      "=>",
      "->",
      "::",
      "==",
      "!=",
      "<=",
      ">=",
      "&&",
      "||",
      "+=",
      "-=",
      "*=",
      "/=",
      "%=",
      /[+\-*\/%=<>!&|.:?]+/,
    )),

    punctuation: (_) => token(/[{}\[\](),;]/),
  },
})
