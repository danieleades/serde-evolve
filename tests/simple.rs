#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use serde_evolve::Versioned;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct V1 {
    pub field: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct V2 {
    pub field: String,
    pub new_field: i32,
}

impl From<V1> for V2 {
    fn from(v1: V1) -> Self {
        Self {
            field: v1.field,
            new_field: 0,
        }
    }
}

#[derive(Versioned)]
#[versioned(mode = "infallible", chain(V1, V2))]
pub struct MyType {
    pub field: String,
    pub new_field: i32,
}

impl Clone for MyType {
    fn clone(&self) -> Self {
        Self {
            field: self.field.clone(),
            new_field: self.new_field,
        }
    }
}

impl std::fmt::Debug for MyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MyType")
            .field("field", &self.field)
            .field("new_field", &self.new_field)
            .finish()
    }
}

impl PartialEq for MyType {
    fn eq(&self, other: &Self) -> bool {
        self.field == other.field && self.new_field == other.new_field
    }
}

impl From<V2> for MyType {
    fn from(v2: V2) -> Self {
        Self {
            field: v2.field,
            new_field: v2.new_field,
        }
    }
}

impl From<&MyType> for V2 {
    fn from(t: &MyType) -> Self {
        Self {
            field: t.field.clone(),
            new_field: t.new_field,
        }
    }
}

#[test]
fn test_basic() {
    let json_v1 = r#"{"_version":"1","field":"test"}"#;
    let rep: MyTypeVersions = serde_json::from_str(json_v1).unwrap();
    assert_eq!(rep.version(), 1);

    let my_type: MyType = rep.into();
    assert_eq!(my_type.field, "test");
    assert_eq!(my_type.new_field, 0);
}
