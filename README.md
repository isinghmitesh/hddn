# hddn

[![Crates.io](https://img.shields.io/crates/v/hddn.svg)](https://crates.io/crates/hddn)
[![Documentation](https://docs.rs/hddn/badge.svg)](https://docs.rs/hddn)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A cross-platform Rust library to check if a file or directory is hidden on Windows, macOS, and Linux.

## Features

- **Cross-platform support**: Works on Windows, macOS, and Linux
- **Native implementations**: Uses OS-specific APIs for accurate detection
- **Simple API**: Just one function to check if a file is hidden
- **Lightweight**: Minimal dependencies

## Platform-Specific Behavior

- **Windows**: Checks the `FILE_ATTRIBUTE_HIDDEN` attribute
- **macOS**: Checks the `UF_HIDDEN` flag and dot-prefix
- **Linux**: Checks if the filename starts with a dot (`.`)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
hddn = "0.1.0"
```

## Usage

### Basic Example

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

### Checking Multiple Files

```rust
use hddn::is_hidden;
use std::path::PathBuf;

fn main() {
    let paths = vec![
        PathBuf::from(".hidden_file"),
        PathBuf::from("normal_file.txt"),
        PathBuf::from("C:\\hidden_folder\\file.txt"), // Windows example
    ];

    for path in paths {
        match is_hidden(&path) {
            Ok(true) => println!("{:?} is hidden", path),
            Ok(false) => println!("{:?} is not hidden", path),
            Err(e) => println!("Error checking {:?}: {:?}", path, e),
        }
    }
}
```

### Filtering Hidden Files

```rust
use hddn::is_hidden;
use std::fs;
use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let entries = fs::read_dir(".")?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| !is_hidden(path).unwrap_or(false))
        .collect::<Vec<PathBuf>>();

    println!("Non-hidden files: {:?}", entries);
    Ok(())
}
```

## How It Works

The library uses platform-specific methods to determine if a file is hidden:

1. **Cross-platform check**: All platforms first check if the filename starts with a dot (`.`), which is a universal convention for hidden files in Unix-like systems.

2. **Windows**: Uses the Win32 API `GetFileAttributesA` to check if the `FILE_ATTRIBUTE_HIDDEN` flag is set on the file.

3. **macOS**: Uses the BSD `stat` function to check if the `UF_HIDDEN` flag is set in the file's `st_flags` field.

4. **Linux**: Relies solely on the dot-prefix convention, as Linux doesn't have a specific "hidden" attribute at the filesystem level.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.