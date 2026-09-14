# Dropworks C# binding

Planned `Dropworks.Client` package for Unity/Godot C# projects.

The binding should P/Invoke the C ABI in
[`../../include/dropworks.h`](../../include/dropworks.h) rather than reimplement
HTTP, so all native bindings share one contract. The `libdropworks` reference
implementation is tracked in
[#19](https://github.com/Heretek-Games/dropworks-sdk/issues/19); this binding
is not implemented yet.
