# Build Performance

## Applied

| Technique | Config | Gain |
|---|---|---|
| **Dynamic linking** | `crates/client/Cargo.toml`: `features = ["bevy_dev_tools", "dynamic_linking"]` | Avoids relinking Bevy on every change — incremental rebuild dropped from 26s → 0.7s |
| **Mold linker** | `.cargo/config.toml` sets `linker = "/tmp/gcc-mold.sh"` (wrapper that invokes mold via `gcc -fuse-ld=mold`) | Faster link phase. Requires `/tmp/gcc-mold.sh` and `/tmp/ld.mold` symlink to be present |
| **Profile optimizations** | `Cargo.toml`: `[profile.dev] opt-level = 1`, `[profile.dev.package."*"] opt-level = 3` | Balances debug compile speed with runtime perf for deps |

## Available but Not Applied

These require switching to **nightly Rust** via `rust-toolchain.toml`:

| Technique | Config | Expected gain |
|---|---|---|
| **Cranelift codegen** | Use Cranelift for workspace crates, LLVM for deps | ~30% faster codegen for own code |
| **Generic sharing** (`-Zshare-generics=y`) | `.cargo/config.toml` rustflags | ~10–20% less duplicate monomorphization |
| **Parallel frontend** (`-Zthreads=0`) | `.cargo/config.toml` rustflags | Faster parsing/typechecking on multi-core |

These are commented out in `.cargo/config.toml` — uncomment when on nightly.

## Caveats

- **Dynamic linking** produces a 2GB `libbevy_dylib.so`. Do not ship with it enabled — use `--release` or a separate profile for release builds.
- **Nightly Rust** may have occasional breakage. Pin a specific nightly with `rust-toolchain.toml`.
- **Mold wrapper** is a temporary path — install mold system-wide (`sudo dnf install mold`) and use `linker = "gcc"` + `-fuse-ld=mold` for a cleaner setup.
