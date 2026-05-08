export const TEMPLATES = {
  "Hello User": `#[derive(Debug, Clone)]
pub struct User
    pub id: u64
    pub name: String

impl User
    pub fn new(id: u64, name: String) -> Self =
        Self
            id = id
            name = name

    pub fn display_name(&self) -> &str = &self.name

pub fn main() =
    let user = User.new(1, "Ada".to_string())
    do println!("{}", user.display_name())
`,
  "Result Flow":
    `pub fn parse_port(raw: &str) -> Result[u16, std.num.ParseIntError] =
    let port = raw.parse[u16]()
    Ok(port)

pub fn main() =
    match parse_port("8080")
        Ok(port) => do println!("listening on {port}")
        Err(error) => do eprintln!("invalid port: {error}")
`,
  "Module Layout": `#![allow(dead_code)]

pub mod math
    pub fn clamp(value: i32, min: i32, max: i32) -> i32 =
        if value < min then min else if value > max then max else value

pub trait Render
    fn render(&self) -> String
`,
} as const

export type TemplateName = keyof typeof TEMPLATES

export const TEMPLATE_NAMES = Object.keys(TEMPLATES) as TemplateName[]
