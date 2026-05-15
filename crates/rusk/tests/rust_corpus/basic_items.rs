#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: u64,
    pub name: String,
}

impl User {
    pub fn new(id: u64, name: String) -> Self {
        Self { id, name }
    }

    pub fn display_name(&self) -> &str {
        &self.name
    }
}

pub fn make_user() -> User {
    User::new(1, "Ada".to_string())
}
