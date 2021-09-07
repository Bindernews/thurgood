
#[cfg(feature = "json")]
pub mod json {
    use serde_json::{Value, Map};

    pub trait JsonMapExt {
        fn ezset<K, V>(&mut self, key: K, value: V) where K: AsRef<str>, V: Into<Value>;
    }
    impl JsonMapExt for Map<String, Value> {
        fn ezset<K, V>(&mut self, key: K, value: V) where K: AsRef<str>, V: Into<Value> {
            self.insert(key.as_ref().to_owned(), value.into());
        }
    }
}