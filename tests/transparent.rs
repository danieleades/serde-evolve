//! Tests for transparent serde mode

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use serde_evolve::Versioned;

// ============================================================================
// Infallible transparent mode tests
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct V1 {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct V2 {
    pub full_name: String,
    pub email: Option<String>,
}

impl From<V1> for V2 {
    fn from(v1: V1) -> Self {
        Self {
            full_name: v1.name,
            email: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Versioned)]
#[versioned(mode = "infallible", chain(V1, V2), transparent = true)]
pub struct User {
    pub full_name: String,
    pub email: Option<String>,
}

impl From<V2> for User {
    fn from(v2: V2) -> Self {
        Self {
            full_name: v2.full_name,
            email: v2.email,
        }
    }
}

impl From<&User> for V2 {
    fn from(user: &User) -> Self {
        Self {
            full_name: user.full_name.clone(),
            email: user.email.clone(),
        }
    }
}

#[test]
fn test_transparent_deserialize_v1() {
    let json_v1 = r#"{"_version":"1","name":"Alice"}"#;

    // With transparent mode, we can deserialize directly to User
    let user: User = serde_json::from_str(json_v1).unwrap();

    assert_eq!(user.full_name, "Alice");
    assert_eq!(user.email, None);
}

#[test]
fn test_transparent_deserialize_v2() {
    let json_v2 = r#"{"_version":"2","full_name":"Bob","email":"bob@example.com"}"#;

    // With transparent mode, we can deserialize directly to User
    let user: User = serde_json::from_str(json_v2).unwrap();

    assert_eq!(user.full_name, "Bob");
    assert_eq!(user.email, Some("bob@example.com".to_string()));
}

#[test]
fn test_transparent_serialize() {
    let user = User {
        full_name: "Charlie".to_string(),
        email: Some("charlie@example.com".to_string()),
    };

    // With transparent mode, we can serialize directly from User
    let json = serde_json::to_string(&user).unwrap();

    // Should serialize as V2 (current version)
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["_version"], "2");
    assert_eq!(parsed["full_name"], "Charlie");
    assert_eq!(parsed["email"], "charlie@example.com");
}

#[test]
fn test_transparent_round_trip() {
    let original = User {
        full_name: "Dave".to_string(),
        email: Some("dave@example.com".to_string()),
    };

    // Serialize
    let json = serde_json::to_string(&original).unwrap();

    // Deserialize
    let deserialized: User = serde_json::from_str(&json).unwrap();

    assert_eq!(original, deserialized);
}

// ============================================================================
// Fallible transparent mode tests
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductV1 {
    pub name: String,
    pub price: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductV2 {
    pub name: String,
    pub price_cents: i64,
}

impl TryFrom<ProductV1> for ProductV2 {
    type Error = anyhow::Error;

    fn try_from(v1: ProductV1) -> Result<Self, Self::Error> {
        if !v1.price.is_finite() {
            anyhow::bail!("Price must be finite");
        }

        let cents = (v1.price * 100.0).round();
        #[allow(clippy::cast_possible_truncation)]
        let price_cents = cents as i64;

        Ok(Self {
            name: v1.name,
            price_cents,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Versioned)]
#[versioned(error = anyhow::Error, chain(ProductV1, ProductV2), transparent = true)]
pub struct Product {
    pub name: String,
    pub price_cents: i64,
}

impl TryFrom<ProductV2> for Product {
    type Error = anyhow::Error;

    fn try_from(v2: ProductV2) -> Result<Self, Self::Error> {
        Ok(Self {
            name: v2.name,
            price_cents: v2.price_cents,
        })
    }
}

impl From<&Product> for ProductV2 {
    fn from(product: &Product) -> Self {
        Self {
            name: product.name.clone(),
            price_cents: product.price_cents,
        }
    }
}

#[test]
fn test_transparent_fallible_deserialize_v1() {
    let json_v1 = r#"{"_version":"1","name":"Widget","price":19.99}"#;

    // With transparent mode, we can deserialize directly to Product
    let product: Product = serde_json::from_str(json_v1).unwrap();

    assert_eq!(product.name, "Widget");
    assert_eq!(product.price_cents, 1999);
}

#[test]
fn test_transparent_fallible_deserialize_v2() {
    let json_v2 = r#"{"_version":"2","name":"Gadget","price_cents":2500}"#;

    // With transparent mode, we can deserialize directly to Product
    let product: Product = serde_json::from_str(json_v2).unwrap();

    assert_eq!(product.name, "Gadget");
    assert_eq!(product.price_cents, 2500);
}

#[test]
fn test_transparent_fallible_serialize() {
    let product = Product {
        name: "Thingamajig".to_string(),
        price_cents: 2499,
    };

    // With transparent mode, we can serialize directly from Product
    let json = serde_json::to_string(&product).unwrap();

    // Should serialize as V2 (current version)
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["_version"], "2");
    assert_eq!(parsed["name"], "Thingamajig");
    assert_eq!(parsed["price_cents"], 2499);
}

#[test]
fn test_transparent_fallible_round_trip() {
    let original = Product {
        name: "Doohickey".to_string(),
        price_cents: 1999,
    };

    // Serialize
    let json = serde_json::to_string(&original).unwrap();

    // Deserialize
    let deserialized: Product = serde_json::from_str(&json).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn test_transparent_fallible_migration_error() {
    // Invalid data that should fail migration
    let json_invalid = r#"{"_version":"1","name":"Invalid","price":null}"#;

    // With transparent mode, migration errors are wrapped in serde errors
    let result: Result<Product, _> = serde_json::from_str(json_invalid);

    assert!(result.is_err());
    // The error message should indicate it's a custom deserialization error
    let err = result.unwrap_err();
    assert!(err.is_data());
}
