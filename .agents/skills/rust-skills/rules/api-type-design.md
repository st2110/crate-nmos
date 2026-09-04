# api-type-design

> Favor composition and encapsulation in struct design

## Why It Matters

Rust has no inheritance. The natural way to reuse behavior is composition: a struct owns or borrows the collaborators it needs. Keeping fields private by default prevents external code from depending on internal representation and lets you change the implementation without breaking callers.

## Prefer Composition Over Inheritance-Like Patterns

Avoid simulating inheritance with deep `Deref` chains, trait extension points, or structs that exist only to wrap a parent "base" struct. Instead, compose smaller types.

```rust
// Bad: trying to model inheritance with wrappers
pub struct BaseEntity {
    pub id: u64,
    pub name: String,
}

pub struct Player {
    pub base: BaseEntity,
    pub score: u32,
}

// Callers reach through layers: player.base.name
```

```rust
// Good: composition with owned fields
pub struct Player {
    id: u64,
    name: String,
    score: u32,
}

impl Player {
    pub fn id(&self) -> u64 { self.id }
    pub fn name(&self) -> &str { &self.name }
    pub fn score(&self) -> u32 { self.score }
}
```

If several types share behavior, use a trait or a small helper type rather than a base struct:

```rust
pub trait Named {
    fn name(&self) -> &str;
}

impl Named for Player {
    fn name(&self) -> &str { &self.name }
}

impl Named for Team {
    fn name(&self) -> &str { &self.name }
}
```

## Fields Private by Default

Make struct fields `pub` only when they are part of the stable public contract. Use accessor methods for read access and constructor/builder methods for write access.

```rust
// Bad: every field is public, internal state leaks
pub struct Counter {
    pub value: i64,
    pub max: i64,
}
```

```rust
// Good: invariants are protected
pub struct Counter {
    value: i64,
    max: i64,
}

impl Counter {
    pub fn new(max: i64) -> Self {
        Self { value: 0, max }
    }

    pub fn value(&self) -> i64 {
        self.value
    }

    pub fn increment(&mut self) -> bool {
        if self.value < self.max {
            self.value += 1;
            true
        } else {
            false
        }
    }
}
```

Public fields are appropriate for plain data carriers where there are no invariants and the type is unlikely to evolve (e.g., a geometry `Point` or a config deserialized from JSON with no behavior).

## See Also

- [api-builder-pattern](./api-builder-pattern.md) - Builder pattern for complex construction
- [api-newtype-safety](./api-newtype-safety.md) - Use newtypes to distinguish values
- [proj-free-functions-vs-methods](./proj-free-functions-vs-methods.md) - Behavior tied to one struct should be a method
