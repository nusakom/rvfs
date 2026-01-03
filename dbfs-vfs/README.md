# dbfs-vfs

DBFS (Database Filesystem) adapter for RVFS framework.

## Overview

`dbfs-vfs` is an integration layer that allows [DBFS2](https://github.com/nusakom/dbfs) to work within the [RVFS](https://github.com/os-module/rvfs) framework, enabling database-backed filesystems in Rust operating systems and applications.

## Features

- **VFS Trait Implementation**: Full implementation of `VfsFsType` trait for seamless integration
- **Multi-instance Support**: Multiple DBFS instances with different databases
- **Git Dependency Management**: Clean separation between DBFS and RVFS repositories
- **Thread-safe**: All operations are thread-safe using Arc and Mutex
- **Zero-copy**: Efficient data access patterns

## Architecture

```
Application
    ↓
RVFS vfscore (VFS Layer)
    ↓
dbfs-vfs (Adapter)
    ↓
dbfs2 (Filesystem Implementation)
    ↓
jammdb (Database Engine)
```

## Usage

### Basic Example

```rust
use dbfs_vfs::DbFsAdapter;
use std::sync::Arc;

// Create DBFS adapter
let dbfs = Arc::new(DbFsAdapter::new("/path/to/database.db".to_string()));

// Mount the filesystem
let root_dentry = dbfs.mount(0, "/mnt/dbfs", None, &[])?;

// Use the filesystem through VFS API
```

### Registration in Application

```rust
use vfscore::fstype::VfsFsType;
use std::collections::HashMap;

fn register_filesystems() -> HashMap<String, Arc<dyn VfsFsType>> {
    let mut registry = HashMap::new();

    // Register DBFS
    let dbfs = Arc::new(DbFsAdapter::new("/tmp/dbfs.db".to_string()));
    registry.insert("dbfs".to_string(), dbfs);

    registry
}
```

### Multiple Instances

```rust
// Different databases for different purposes
let dbfs_data = Arc::new(DbFsAdapter::new("/var/lib/dbfs/data.db".to_string()));
let dbfs_log = Arc::new(DbFsAdapter::new("/var/lib/dbfs/log.db".to_string()));
let dbfs_cache = Arc::new(DbFsAdapter::new("/var/lib/dbfs/cache.db".to_string()));
```

## Building

```bash
# Build without DBFS (type checking only)
cargo build

# Build with DBFS support
cargo build --features dbfs

# Run examples
cargo run --example integration_example --features dbfs
```

## Dependencies

- `vfscore` - VFS framework (local workspace dependency)
- `dbfs2` - DBFS filesystem implementation (optional, Git dependency)
- `log` - Logging facade

## Documentation

See [INTEGRATION.md](INTEGRATION.md) for detailed integration instructions and examples.

## Repository Structure

```
dbfs-vfs/
├── Cargo.toml              # Package configuration
├── src/
│   └── lib.rs              # Main implementation
├── examples/
│   └── integration_example.rs  # Usage examples
├── INTEGRATION.md          # Integration guide
└── README.md              # This file
```

## Features Flags

- `dbfs`: Enable DBFS2 integration (requires Git dependencies)
- `default`: No features enabled by default

## License

Same as RVFS framework.

## Contributing

Contributions are welcome! Please ensure:
- Code follows Rust style guidelines
- All tests pass (`cargo test`)
- Documentation is updated
- Git commit messages follow conventional format

## Related Projects

- [RVFS](https://github.com/os-module/rvfs) - Virtual Filesystem framework
- [DBFS2](https://github.com/nusakom/dbfs) - Database filesystem implementation
- [jammdb](https://github.com/nusakom/jammdb) - Embedded database engine
