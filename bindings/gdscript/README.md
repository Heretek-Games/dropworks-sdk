# Dropworks GDScript binding

Planned `Dropworks` autoload singleton for Godot 4.

The singleton should be backed by a GDExtension that wraps the C ABI in
[`../../include/dropworks.h`](../../include/dropworks.h), keeping the same
contract as the C, C#, and Rust bindings. The `libdropworks` reference
implementation is tracked in
[#19](https://github.com/Heretek-Games/dropworks-sdk/issues/19); this binding
is not implemented yet.
