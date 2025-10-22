# serde-evolve - Type-Safe Data Schema Evolution

[![Documentation](https://docs.rs/serde-evolve/badge.svg)](https://docs.rs/serde-evolve)

A Rust library for versioning serialised data structures with compile-time verified migrations.

## Overview

`serde-evolve` helps you evolve data schemas over time while maintaining backward compatibility with historical data. It separates **wire format** (serialization) from **domain types** (application logic), allowing you to deserialise any historical version and migrate it to your current domain model.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
serde-evolve = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

## Key Features

- ✅ **Compile-time safety**: Type-checked migration chains
- ✅ **Standard Rust traits**: Uses `From`/`TryFrom`, no custom APIs
- ✅ **Clean separation**: Representation types stay separate from domain logic
- ✅ **Framework-agnostic**: Works with any serde format (JSON, bincode, etc.)
- ✅ **Fallible migrations**: Support validation and transformation errors
- ✅ **Simple macro**: Generate boilerplate using a derive macro

## Quick Example

```rust
use serde::{Deserialize, Serialize};
use serde_evolve::Versioned;

// Define version DTOs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserV1 {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserV2 {
    pub full_name: String,
    pub email: Option<String>,
}

// Define migrations
impl From<UserV1> for UserV2 {
    fn from(v1: UserV1) -> Self {
        Self {
            full_name: v1.name,
            email: None,
        }
    }
}

// Define domain type
#[derive(Clone, Debug, Versioned)]
#[versioned(
    mode = "infallible",
    chain(UserV1, UserV2),
)]
pub struct User {
    pub full_name: String,
    pub email: Option<String>,
}

// Final migration to domain
impl From<UserV2> for User {
    fn from(v2: UserV2) -> Self {
        Self {
            full_name: v2.full_name,
            email: v2.email,
        }
    }
}

// Serialization (domain → representation)
impl From<&User> for UserV2 {
    fn from(user: &User) -> Self {
        Self {
            full_name: user.full_name.clone(),
            email: user.email.clone(),
        }
    }
}

// Usage:
fn main() -> Result<(), Box<dyn std::error::Error>> {
let json_v1 = r#"{"_version":"1","name":"Alice"}"#;
let rep: UserVersions = serde_json::from_str(json_v1)?;
let user: User = rep.into(); // Automatic migration V1 → V2 → User
Ok(())
}
```

## Modes

### Infallible Mode

All migrations guaranteed to succeed:

```rust
#[versioned(mode = "infallible", chain(V1, V2))]
```

Generates: `impl From<Representation> for Domain`

### Fallible Mode

Migrations can fail (validation, transformation errors):

```rust
// `mode = "fallible"` is the default; specify it only when overriding.
#[versioned(error = MyError, chain(V1, V2))]
```

Generates: `impl TryFrom<Representation> for Domain`

## Transparent Serde Support

By default, you work explicitly with the representation enum:

```rust,ignore
// Default behavior - explicit representation
let rep: UserVersions = serde_json::from_str(json)?;
let user: User = rep.try_into()?;
```

The `transparent = true` flag generates custom `Serialize`/`Deserialize` implementations that allow direct domain type serialisation:

```rust,ignore
#[versioned(
    mode = "infallible",
    chain(V1, V2),
    transparent = true  // ← Enable transparent serde
)]
pub struct User {
    pub name: String,
}

// Now works directly:
let user: User = serde_json::from_str(json)?;
let json = serde_json::to_string(&user)?;
```

## Representation Format

Data is serialised with an embedded `_version` tag:

```json
{
  "_version": "1",
  "name": "Alice"
}
```

Serde's `#[serde(tag = "_version")]` handles routing to the correct variant.

## Design Principles

1. **Representation/Domain Separation**: Domain types never leak serialisation concerns
2. **Standard Traits**: Uses Rust's `From`/`TryInto`, not custom APIs
3. **Type Safety**: Missing migrations cause compile errors
4. **User Control**: You define all version structs and migrations

## Architecture

```text
┌─────────────────────────────────────────────────┐
│  Historical Data (V1, V2, ...)                  │
└────────────────┬────────────────────────────────┘
                 │ Deserialize
                 ▼
┌─────────────────────────────────────────────────┐
│  Representation Enum (auto-generated)          │
│  ┌─────────────────────────────────────────┐   │
│  │ enum UserVersions {                     │   │
│  │   V1(UserV1),                           │   │
│  │   V2(UserV2),                           │   │
│  │ }                                       │   │
│  └─────────────────────────────────────────┘   │
└────────────────┬────────────────────────────────┘
                 │ From/TryFrom (chain migrations)
                 ▼
┌─────────────────────────────────────────────────┐
│  Domain Type (your application logic)           │
│  struct User { ... }                            │
└─────────────────────────────────────────────────┘
```

## Generated Code

The `#[derive(Versioned)]` macro generates:

1. **Representation enum** with serde tags
2. **`From<Representation> for Domain`** (or `TryFrom` for fallible)
3. **`From<&Domain> for Representation`** (for serialization)
4. **Helper methods**: `version()`, `is_current()`, `CURRENT`

## Use Cases

- **Event sourcing**: Immutable event streams that must be replayable
- **Message queues**: Long-lived messages with evolving schemas
- **API versioning**: Supporting multiple client versions
- **Data archives**: Historical records that must remain accessible
- **Configuration files**: Version migrations for user settings
