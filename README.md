# lil-json

`#![no_std]` Rust crate to parse & serialize JSON

the following types are currently supported:
* objects (currently limited to non-nested objects)
* string (currently limited to ascii)
* boolean
* number

the following types are not currently supported:
* arrays
* null
* nested types

TODO:
- [ ] support null type
- [ ] expose serialization methods for terminal types
- [ ] support arrays
- [ ] support parsing arbitrary types
- [ ] support unicode strings
