//! Simple example demonstrating infallible migrations.
//!
//! Run with: `cargo run --example simple_infallible`

#![allow(missing_docs)]

use serde_evolve::Versioned;

// ============================================================================
// Domain Type (with versioning macro applied)
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq, Versioned)]
#[versioned(mode = "infallible", chain(versions::V1, versions::V2))]
pub struct User {
    pub full_name: String,
    pub email: Option<String>,
}

mod versions {
    use super::User;
    use serde::{Deserialize, Serialize};

    // ============================================================================
    // Version DTOs (serialized representation)
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

    // ============================================================================
    // Migrations
    // ============================================================================

    // V1 -> V2: rename 'name' to 'full_name', add email field
    impl From<V1> for V2 {
        fn from(v1: V1) -> Self {
            Self {
                full_name: v1.name,
                email: None,
            }
        }
    }

    // V2 -> Domain: straightforward conversion
    impl From<V2> for User {
        fn from(v2: V2) -> Self {
            Self {
                full_name: v2.full_name,
                email: v2.email,
            }
        }
    }

    // Domain -> V2: for serialization
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
// Example usage
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Versioned Data: Simple Infallible Example ===\n");

    // ========================================
    // Deserialize V1 data
    // ========================================
    println!("1. Reading V1 data...");
    let json_v1 = r#"{"_version":"1","name":"Alice"}"#;
    println!("   JSON: {json_v1}");

    let rep_v1: UserVersions = serde_json::from_str(json_v1)?;
    println!("   Version: {}", rep_v1.version());

    let user_from_v1: User = rep_v1.into();
    println!("   User: {user_from_v1:?}\n");

    assert_eq!(user_from_v1.full_name, "Alice");
    assert_eq!(user_from_v1.email, None);

    // ========================================
    // Deserialize V2 data
    // ========================================
    println!("2. Reading V2 data...");
    let json_v2 = r#"{"_version":"2","full_name":"Bob","email":"bob@example.com"}"#;
    println!("   JSON: {json_v2}");

    let rep_v2: UserVersions = serde_json::from_str(json_v2)?;
    println!("   Version: {}", rep_v2.version());
    println!("   Is current: {}", rep_v2.is_current());

    let user_from_v2: User = rep_v2.into();
    println!("   User: {user_from_v2:?}\n");

    assert_eq!(user_from_v2.full_name, "Bob");
    assert_eq!(user_from_v2.email, Some("bob@example.com".to_string()));

    // ========================================
    // Serialize current data
    // ========================================
    println!("3. Writing current version...");
    let charlie = User {
        full_name: "Charlie".to_string(),
        email: Some("charlie@example.com".to_string()),
    };

    let rep_charlie: UserVersions = (&charlie).into();
    println!("   Writing as version: {}", rep_charlie.version());
    println!("   Current version: {}", UserVersions::CURRENT);

    let json_charlie = serde_json::to_string_pretty(&rep_charlie)?;
    println!("   JSON:\n{json_charlie}\n");

    // ========================================
    // Verify round-trip
    // ========================================
    println!("4. Verifying round-trip...");
    let rep_back: UserVersions = serde_json::from_str(&json_charlie)?;
    let charlie_back: User = rep_back.into();

    assert_eq!(charlie, charlie_back);
    println!("   ✓ Round-trip successful!\n");

    println!("=== Example completed successfully! ===");
    Ok(())
}
