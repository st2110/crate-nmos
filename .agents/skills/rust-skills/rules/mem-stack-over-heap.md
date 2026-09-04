# mem-stack-over-heap

> Prefer the stack when the size is known and small

## Why It Matters

Heap allocation is more expensive than stack allocation: it requires acquiring an allocator lock, finding a free block, and later returning to the allocator. Stack memory is essentially free (a pointer bump) and is automatically reclaimed when the function returns. For small, fixed-size, short-lived data, the stack is usually the right place.

## Bad

```rust
// Allocates on the heap for a tiny fixed array
fn sum_pair(values: (i32, i32)) -> i32 {
    let vec = vec![values.0, values.1];  // Unnecessary heap allocation
    vec.into_iter().sum()
}

// Allocates a String just to compare a constant prefix
fn starts_with_http(url: &str) -> bool {
    let prefix = String::from("http");  // Heap allocation for static text
    url.starts_with(&prefix)
}
```

## Good

```rust
// Stack-allocated array
fn sum_pair(values: (i32, i32)) -> i32 {
    let arr = [values.0, values.1];
    arr.into_iter().sum()
}

// Static string slice
fn starts_with_http(url: &str) -> bool {
    url.starts_with("http")
}
```

## Fixed-Size Arrays and Small Const Generics

```rust
// A 3x3 matrix is fixed and small; keep it on the stack
pub struct Matrix3x3([[f64; 3]; 3]);

// Small buffers can use const generics
pub struct RingBuffer<T, const N: usize> {
    data: [T; N],
    head: usize,
}
```

## When the Heap Is Right

Use the heap when:
- the size is only known at runtime,
- the value must outlive the current stack frame,
- the collection may grow arbitrarily,
- the value is large enough to risk stack overflow.

```rust
// Runtime size: Vec is correct
fn collect_lines(reader: impl BufRead) -> Vec<String> { ... }

// Large buffer that must outlive the function
let big = Box::new([0u8; 1_000_000]);
```

## See Also

- [mem-arrayvec](./mem-arrayvec.md) - Fixed-capacity collections without heap allocation
- [mem-smallvec](./mem-smallvec.md) - Use SmallVec for usually-small collections
- [own-slice-over-vec](./own-slice-over-vec.md) - Accept slices, not vectors
