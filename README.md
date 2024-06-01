# hddn

A Rust library to check if a file is hidden on Windows, macOS, and Linux.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
hddn = "0.1.0"


## Example
```rust
use hddn::is_hidden;

fn main() {
    let path = "example.txt";
    match is_hidden(path) {
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