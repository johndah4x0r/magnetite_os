# magnetite_os/common
Definitions that are reasonably expected to be shared between 
[`magnetite_os/boot`](../boot) and [`magnetite_os/kern`](../kern)

(interactive - up one level: [`magnetite_os/`](../))

## Organization
This Rust module is currently divided into three categories:

- `arch` - ISA-specific definitions
- `plat` - Platform-specific definitions
- `shared` - Platform-agnostic definitions

## Documentation
The documentation for the crate `common` can be generated and viewed by running
```bash
cargo doc --open
```
inside the directory `common/` (which should be **this** directory).

## Rationale
Although it is entirely possible to move on with the project by simply 
placing definitions where they need to be, it would be nice to:

- have a single known location for the definitions
- be able to share the definitions wherever possible, and
- distinguish between target-agnostic and target-specific definitions
