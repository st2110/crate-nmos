# test-use-super

> Use `use super::*;` in inline test modules to access parent module items

## Why It Matters

An inline `#[cfg(test)] mod tests` is a child of the module it tests. `use super::*;` imports everything from the parent module, including private items, so tests can exercise both public API and internal helpers without verbose or brittle imports.

## Bad

```rust
// src/parser.rs
pub fn parse(input: &str) -> Result<Ast, Error> { ... }
fn tokenize(input: &str) -> Result<Vec<Token>, Error> { ... }

#[cfg(test)]
mod tests {
    // Verbose, misses private items, and breaks easily on renames.
    use crate::parser::parse;

    #[test]
    fn parses_simple_expression() {
        let ast = parse("1 + 2").unwrap();
        assert_eq!(ast.value(), 3);
    }
}
```

## Good

```rust
// src/parser.rs
pub fn parse(input: &str) -> Result<Ast, Error> { ... }
fn tokenize(input: &str) -> Result<Vec<Token>, Error> { ... }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_expression() {
        let ast = parse("1 + 2").unwrap();
        assert_eq!(ast.value(), 3);
    }

    #[test]
    fn tokenizes_operators() {
        // Can access the private `tokenize` function.
        let tokens = tokenize("1 + 2").unwrap();
        assert_eq!(tokens.len(), 3);
    }
}
```

## Nested Modules

```rust
mod outer {
    pub fn outer_fn() -> i32 { 1 }

    mod inner {
        pub fn inner_fn() -> i32 { 2 }

        #[cfg(test)]
        mod tests {
            use super::*;        // inner's items
            use super::super::*; // outer's items

            #[test]
            fn test_inner() {
                assert_eq!(inner_fn(), 2);
                assert_eq!(outer_fn(), 1);
            }
        }
    }
}
```

## When Not to Use

This rule applies to **inline** `#[cfg(test)]` modules. For tests in separate files (`tests/`, `*_suite.rs`, `..._tests.rs`), use absolute imports such as `use crate::...` or `use my_crate::...` instead.

## See Also

- [test-descriptive-names](./test-descriptive-names.md) - Use descriptive test names
- [test-arrange-act-assert](./test-arrange-act-assert.md) - Structure tests with AAA
