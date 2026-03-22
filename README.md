# e64-rust

Minimal x86_64 "hello world" kernel built in Rust and loaded by Limine.

## Prerequisites
- Rust toolchain with the `x86_64-unknown-none` target (`rustup target add x86_64-unknown-none`).
- `curl`, `gcc`/`clang`, `nasm`, `make`, `mtools`, `xorriso`, `llvm`, `mawk`
- `qemu-system-x86_64` for running the ISO.

## Building
```bash
# Build the kernel and a Limine hybrid ISO (downloads and builds Limine v10.3.2)
cargo run -p xtask -- iso
```

Artifacts:
- Kernel ELF: `target/x86_64-unknown-none/release/e64-rust`
- Bootable ISO: `build/e64-rust.iso`

## Running in QEMU
```bash
cargo run -p xtask -- run        # extra QEMU args can be appended after `run`
```

## Notes
- The kernel draws directly to the Limine-provided framebuffer; see `src/main.rs`.
- Bootloader configuration lives in `limine.conf`.
- Adjust `LIMINE_VERSION` in the `Makefile` if you want a different Limine release.
