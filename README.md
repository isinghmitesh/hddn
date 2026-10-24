# hddn

A Rust library to check if a file is hidden on Windows, macOS, and Linux.

A name that starts with a dot is hidden everywhere. macOS also reads the
`UF_HIDDEN` flag (`chflags hidden`), and Windows reads `FILE_ATTRIBUTE_HIDDEN`
(`attrib +h`). Paths are passed to the OS as they are stored, so names that
aren't valid UTF-8 work on macOS, and names with any Unicode character work on
Windows.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
hddn = "0.1.0"
```


## Example
```rust
use hddn::is_hidden;
use std::path::PathBuf;

fn main() {
    let path = PathBuf::from("example.txt");
    match is_hidden(&path) {
        Ok(hidden) => {
            if hidden {
                println!("The file is hidden.");
            } else {
                println!("The file is not hidden.");
            }
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}

```