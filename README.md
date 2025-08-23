# lil-json

lil `#![no_std]` Rust crate to parse & serialize JavaScript Object Notation (JSON). alloc optional. std optional.

JSON can be serialized into any type that implements [`embedded_io::Write`](https://docs.rs/embedded-io/latest/embedded_io/trait.Write.html).

Here is a minimal example of serializing a JSON object to stdout with a one-liner, making use of the `lil-json::FieldBuffer` auto trait, `core::convert::From for JsonValue`, & `core::convert::Into for JsonField`
```rust
use std::io::stdout;
use lil_json::FieldBuffer;

fn main() {
    [
        ("some_number", 12345).into(),
        ("some_string", "hello world!").into(),
        ("some_boolean", true).into()
    ]
    .as_json_object()
    .serialize_std(stdout())
    .unwrap();
}

// output: {"some_number":12345,"some_string":"hello world!","some_boolean":true}
```

Still a work in progress. Not everything is complete. String escaping is not fully not completed yet.

the following types are currently supported:
* objects (currently limited to non-nested objects)
* string (currently limited to ascii-)
* boolean
* null
* number (currently limited to integers)

the following types are not currently supported:
* arrays
* null
* nested types

TODO:
- [x] support null type
- [ ] alloc features
- [ ] expose serialization methods for terminal types
- [ ] support arrays
- [ ] support parsing arbitrary types
- [ ] support unicode strings
- [ ] support buffered serialization
- [ ] support parsing from stream
- [ ] support parsing streaming objects/arrays
- [ ] support embedded-io-async?
