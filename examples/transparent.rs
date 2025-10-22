//! Example demonstrating transparent serde mode.
//!
//! With `transparent = true`, you can serialize/deserialize domain types directly
//! without manually converting through the representation enum.
//!
//! Compare with `infallible.rs` to see the explicit (default) approach.
//!
//! Run with: `cargo run --example transparent`

#![allow(missing_docs)]

use serde_evolve::Versioned;

// ============================================================================
// Domain Type with Transparent Serde
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq, Versioned)]
#[versioned(
    mode = "infallible",
    chain(versions::V1, versions::V2),
    transparent = true  // ← Enable transparent serde support
)]
pub struct User {
    pub full_name: String,
    pub email: Option<String>,
}

mod versions {
    use super::User;
    use serde::{Deserialize, Serialize};

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
}

// ============================================================================
// Example Usage
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Transparent Serde Mode Example ===\n");

    // ========================================
    // Direct serialization/deserialization
    // ========================================
    println!("1. Serializing current version...");
    let user = User {
        full_name: "Alice".to_string(),
        email: Some("alice@example.com".to_string()),
    };

    // ✅ Serialize directly from domain type (no manual conversion!)
    let json = serde_json::to_string_pretty(&user)?;
    println!("   JSON:\n{json}\n");

    // ✅ Deserialize directly to domain type (no manual conversion!)
    let deserialized: User = serde_json::from_str(&json)?;
    println!("   Deserialized: {deserialized:?}");
    assert_eq!(user, deserialized);

    // ========================================
    // Transparent migration from old versions
    // ========================================
    println!("\n2. Migrating V1 data...");
    let json_v1 = r#"{"_version":"1","name":"Bob"}"#;
    println!("   V1 JSON: {json_v1}");

    // ✅ Migration happens automatically during deserialization
    let migrated: User = serde_json::from_str(json_v1)?;
    println!("   Migrated to: {migrated:?}");
    assert_eq!(migrated.full_name, "Bob");
    assert_eq!(migrated.email, None);

    println!("\n3. Migrating V2 data...");
    let json_v2 = r#"{"_version":"2","full_name":"Charlie","email":"charlie@example.com"}"#;
    println!("   V2 JSON: {json_v2}");

    let migrated: User = serde_json::from_str(json_v2)?;
    println!("   Migrated to: {migrated:?}");
    assert_eq!(migrated.full_name, "Charlie");
    assert_eq!(migrated.email, Some("charlie@example.com".to_string()));

    // ========================================
    // Round-trip verification
    // ========================================
    println!("\n4. Verifying round-trip...");
    let original = User {
        full_name: "Dave".to_string(),
        email: Some("dave@example.com".to_string()),
    };

    let json = serde_json::to_string(&original)?;
    let round_trip: User = serde_json::from_str(&json)?;

    assert_eq!(original, round_trip);
    println!("   ✓ Round-trip successful!\n");

    println!("=== Example completed successfully! ===");
    println!("\nNote: Compare with `infallible.rs` to see the difference.");
    println!("      The explicit mode gives you access to version metadata,");
    println!("      while transparent mode prioritizes API ergonomics.");
    Ok(())
}
