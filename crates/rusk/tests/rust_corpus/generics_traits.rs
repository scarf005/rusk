pub trait Named {
    fn name(&self) -> &str;
}

pub struct Boxed<T> {
    pub value: T,
}

impl<T> Boxed<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }

    pub fn get(&self) -> &T {
        &self.value
    }
}

impl Named for Boxed<String> {
    fn name(&self) -> &str {
        &self.value
    }
}

pub fn clone_twice<T>(value: T) -> (T, T) where T: Clone {
    (value.clone(), value)
}
