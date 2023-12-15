Build
-----
```
default: cargo build --release
linux: cargo build --release --target x86_64-unknown-linux-musl
windows: cargo build --release --target x86_64-pc-windows-msvc
```

Strip
-------
```
strip --strip-debug --remove-section=.comment --remove-section=.note --remove-section=.eh_frame --remove-section=.eh_frame_hdr  --remove-section=.gcc_except_table  --remove-section=.data-rel.ro --remove-section=.note.gnu.build-id --remove-section=.fini --remove-section=.fini_array rust_socks_server -o striped_executeable
```
