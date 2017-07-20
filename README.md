matroska
========

A Rust library for reading metadata from Matroska files
(.mkv, .webm, etc.).

This library supports much of the same metadata reported by
mkvinfo such as the file's title, duration, track information,
attachments, and so on.

## Usage

Add this to your `Cargo.toml`

```toml
[dependencies]
matroska = "0.3"
```

and this to your crate root:

```rust
extern crate matroska;
```
