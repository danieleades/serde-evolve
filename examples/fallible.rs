//! Simple example demonstrating fallible migrations.
//!
//! This example shows how to handle migrations that can fail due to validation
//! or data constraints. Unlike infallible migrations (which use `From`), fallible
//! migrations use `TryFrom` and return `Result` types.
//!
//! Run with: `cargo run --example simple_fallible`

#![allow(missing_docs)]

use serde_evolve::Versioned;

// ============================================================================
// Domain Type (with versioning macro applied)
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq, Versioned)]
#[versioned(
    // mode = "fallible", // this is the default
    error = anyhow::Error,
    chain(versions::V1, versions::V2, versions::V3)
)]
pub struct Product {
    pub name: String,
    pub price_cents: u32,
    pub sku: String,
}

mod versions {
    use super::Product;
    use serde::{Deserialize, Serialize};

    // ============================================================================
    // Version DTOs (serialized representation)
    // ============================================================================

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct V1 {
        pub name: String,
        pub price: f64, // Price stored as float (problematic!)
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct V2 {
        pub name: String,
        pub price_cents: i64, // Fixed: using cents to avoid floating point issues
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct V3 {
        pub name: String,
        pub price_cents: u32, // Fixed: prices can't be negative
        pub sku: String,      // New required field
    }

    // ============================================================================
    // Migration Errors
    // ============================================================================

    #[allow(clippy::cast_precision_loss)]
    const MAX_I64_AS_F64: f64 = i64::MAX as f64;
    #[allow(clippy::cast_precision_loss)]
    const MIN_I64_AS_F64: f64 = i64::MIN as f64;

    // ============================================================================
    // Migrations (all fallible)
    // ============================================================================

    // V1 -> V2: convert float price to cents
    // This can fail if the price is invalid (NaN, infinite, or results in overflow)
    impl TryFrom<V1> for V2 {
        type Error = anyhow::Error;

        fn try_from(v1: V1) -> Result<Self, Self::Error> {
            // Validate the price
            if !v1.price.is_finite() {
                anyhow::bail!("Price must be finite, got: {}", v1.price);
            }

            // Convert to cents (with rounding)
            let cents = (v1.price * 100.0).round();

            // Check for overflow
            if !(MIN_I64_AS_F64..=MAX_I64_AS_F64).contains(&cents) {
                anyhow::bail!("Price too large to convert: {}", v1.price);
            }

            #[allow(clippy::cast_possible_truncation)]
            let price_cents = cents as i64;

            Ok(Self {
                name: v1.name,
                price_cents,
            })
        }
    }

    // V2 -> V3: add SKU and convert to unsigned price
    // This can fail if the price is negative or if we can't generate a valid SKU
    impl TryFrom<V2> for V3 {
        type Error = anyhow::Error;

        fn try_from(v2: V2) -> Result<Self, Self::Error> {
            // Ensure price is non-negative
            if v2.price_cents < 0 {
                anyhow::bail!("Negative price not allowed: {}", v2.price_cents);
            }

            // Generate a SKU from the product name
            // This is a simplified approach - real systems would have more sophisticated SKU generation
            let sku = generate_sku(&v2.name)?;

            let price_cents = u32::try_from(v2.price_cents).map_err(|_| {
                anyhow::anyhow!(
                    "Price does not fit in unsigned representation: {}",
                    v2.price_cents
                )
            })?;

            Ok(Self {
                name: v2.name,
                price_cents,
                sku,
            })
        }
    }

    // V3 -> Domain: straightforward conversion (infallible)
    impl TryFrom<V3> for Product {
        type Error = anyhow::Error;

        fn try_from(v3: V3) -> Result<Self, Self::Error> {
            Ok(Self {
                name: v3.name,
                price_cents: v3.price_cents,
                sku: v3.sku,
            })
        }
    }

    // Domain -> V3: for serialization (infallible)
    impl From<&Product> for V3 {
        fn from(product: &Product) -> Self {
            Self {
                name: product.name.clone(),
                price_cents: product.price_cents,
                sku: product.sku.clone(),
            }
        }
    }

    // ============================================================================
    // Helper Functions
    // ============================================================================

    fn generate_sku(name: &str) -> anyhow::Result<String> {
        // Simple SKU generation: uppercase first 3 chars + random suffix
        let prefix: String = name
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(3)
            .collect::<String>()
            .to_uppercase();

        if prefix.is_empty() {
            anyhow::bail!("Product name contains no valid characters for SKU");
        }

        // In a real system, you'd generate a unique suffix
        // For this example, we'll just pad with zeros
        Ok(format!("{prefix:0<8}"))
    }
}

// ============================================================================
// Example usage
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Versioned Data: Simple Fallible Example ===\n");

    // ========================================
    // Successful migrations
    // ========================================
    println!("1. Successfully migrating V1 data...");
    let json_v1 = r#"{"_version":"1","name":"Widget","price":19.99}"#;
    println!("   JSON: {json_v1}");

    let rep_v1: ProductVersions = serde_json::from_str(json_v1)?;
    println!("   Version: {}", rep_v1.version());

    // Try to convert - this should succeed
    match Product::try_from(rep_v1) {
        Ok(product) => {
            println!("   ✓ Migration successful!");
            println!("   Product: {product:?}\n");
            assert_eq!(product.name, "Widget");
            assert_eq!(product.price_cents, 1999);
            assert_eq!(product.sku, "WID00000");
        }
        Err(e) => {
            println!("   ✗ Migration failed: {e}\n");
            return Err(e.into());
        }
    }

    // ========================================
    // Successful migration: V2 data
    // ========================================
    println!("2. Successfully migrating V2 data...");
    let json_v2 = r#"{"_version":"2","name":"Gadget","price_cents":2500}"#;
    println!("   JSON: {json_v2}");

    let rep_v2: ProductVersions = serde_json::from_str(json_v2)?;
    println!("   Version: {}", rep_v2.version());

    match Product::try_from(rep_v2) {
        Ok(product) => {
            println!("   ✓ Migration successful!");
            println!("   Product: {product:?}\n");
            assert_eq!(product.name, "Gadget");
            assert_eq!(product.price_cents, 2500);
            assert_eq!(product.sku, "GAD00000");
        }
        Err(e) => {
            println!("   ✗ Migration failed: {e}\n");
            return Err(e.into());
        }
    }

    // ========================================
    // Failed migration: negative price
    // ========================================
    println!("3. Attempting to migrate V2 data with negative price...");
    let json_negative = r#"{"_version":"2","name":"Doohickey","price_cents":-500}"#;
    println!("   JSON: {json_negative}");

    let rep_negative: ProductVersions = serde_json::from_str(json_negative)?;
    match Product::try_from(rep_negative) {
        Ok(_) => {
            println!("   ✗ Unexpected success!\n");
            return Err("Expected migration to fail with negative price".into());
        }
        Err(e) => {
            println!("   ✓ Migration correctly failed: {e}\n");
        }
    }

    // ========================================
    // Failed migration: invalid SKU generation
    // ========================================
    println!("4. Attempting to migrate V2 data with invalid product name...");
    let json_invalid_name = r#"{"_version":"2","name":"!!!","price_cents":1000}"#;
    println!("   JSON: {json_invalid_name}");

    let rep_invalid_name: ProductVersions = serde_json::from_str(json_invalid_name)?;
    match Product::try_from(rep_invalid_name) {
        Ok(_) => {
            println!("   ✗ Unexpected success!\n");
            return Err("Expected migration to fail with invalid product name".into());
        }
        Err(e) => {
            println!("   ✓ Migration correctly failed: {e}\n");
        }
    }

    // ========================================
    // Serialize current data
    // ========================================
    println!("5. Writing current version...");
    let thingamajig = Product {
        name: "Thingamajig".to_string(),
        price_cents: 2499,
        sku: "THI12345".to_string(),
    };

    let rep_thingamajig: ProductVersions = (&thingamajig).into();
    println!("   Writing as version: {}", rep_thingamajig.version());
    println!("   Current version: {}", ProductVersions::CURRENT);

    let json_thingamajig = serde_json::to_string_pretty(&rep_thingamajig)?;
    println!("   JSON:\n{json_thingamajig}\n");

    // ========================================
    // Verify round-trip
    // ========================================
    println!("6. Verifying round-trip...");
    let rep_back: ProductVersions = serde_json::from_str(&json_thingamajig)?;
    let thingamajig_back: Product = rep_back.try_into()?;

    assert_eq!(thingamajig, thingamajig_back);
    println!("   ✓ Round-trip successful!\n");

    println!("=== Example completed successfully! ===");
    println!("\nKey takeaways:");
    println!("  • Fallible migrations use TryFrom instead of From");
    println!("  • Validation errors are caught during migration, not serialization");
    println!("  • Invalid historical data can be identified and handled gracefully");
    println!("  • The latest version can still be serialized/deserialized normally");
    Ok(())
}
