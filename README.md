Build
-----
```
default: cargo build --release
linux: cargo build --release --target x86_64-unknown-linux-musl
windows: cargo build --release --target x86_64-pc-windows-msvc
```
