# Thurgood &emsp; [![Latest Version]][crates.io]

[Latest Version]: https://img.shields.io/crates/v/thurgood.svg
[crates.io]: https://crates.io/crates/thurgood

Thurgood is a Rust library that implements (de)serialization for Ruby's Marshal format.

The primary use-case of Thurgood is to load some data, manipulate parts of it, then serailize
the modified data. This includes game saves, stored Rails data, or anything else stored
using `Marshal.dump`. Because of this, Thurgood places a high priority on the ability to
deserialize and re-serialize Marshal streams with as little information-loss as possible.
In many cases Thurgood can deserialize and re-serialize a stream and produce the same output,
byte for byte (see the documentation for exceptions).

Thurgood also provides a convenient method to convert an `RbAny` into a `serde_json::Value`
making it easier to explore or visualize unfamiliar data. Unfortunately this conversion is
one-way, and can fail. See the documentation for more information.

# Status
Thurgood is currently in alpha. It's been successfully tested on some use-cases, but needs
a more extensive suite of unit tests. Furthermore the separation of `thurgood::rc` and
`thurgood::arc` is subject to change, as is the use of reference-counting internally.

