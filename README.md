# Rust Virtual File System (RVFS)

A high-performance, flexible Virtual File System (VFS) abstraction layer for Rust, designed for use in both OS kernels and userspace applications.

## 🚀 Recent Update: DBFS-VFS Integration Specialized
The **DBFS (Database-based File System)** is now fully integrated into the VFS ecosystem. DBFS provides a persistent, database-backed filesystem using `jammdb` as the storage engine. 

### Key Integration Components:
- **DBFS-VFS Adapter Layer**: Bridges the `dbfs2` engine with `vfscore` traits.
- **Thread-Safety (SafeDb)**: Implements a `SafeDb` wrapper to ensure `jammdb` handles are safely shared across multi-threaded VFS operations.
- **VfsInode & VfsFile**: Full implementation of POSIX-compliant operations including `read`, `write`, `readdir`, `lookup`, `unlink`, and `truncate`.

## 📦 Features
- [x] **RamFs**: In-memory filesystem.
- [x] **DevFs**: Device node management.
- [x] **DynFs**: Dynamic filesystem for `/proc` and `/sys` style nodes.
- [x] **DBFS**: Persistent database-backed filesystem using KV storage.
- [x] **VfsCore**: The unified abstraction layer for mount management and path resolution.
- [x] **ExtFs / FatFs**: Support via external providers.

## 🛠 Prerequisites
To build and run the project, especially with DBFS or ExtFS support, you need the following system dependencies:
- **Rust Toolchain** (Nightly recommended for some features)
- **CMake**: Required for building low-level filesystem libraries.
- **Clang / libclang-dev**: Required for `bindgen` to generate bindings for C dependencies.

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y cmake libclang-dev
```

## 🚀 Quick Start (Demo)

The demo application showcases the capabilities of RVFS, including cross-filesystem mounts, symlinks, and the new DBFS persistence/concurrency tests.

```bash
# Run the demo with DBFS enabled
RUST_LOG=info cargo run -p demo --release --features dbfs-vfs/fuse
```

### What the demo tests:
1. **Standard POSIX Operations**: Creating files and directories across RamFs and DBFS.
2. **Persistence**: Demonstrates data integrity across unmount/remount cycles in DBFS.
3. **Concurrency**: Verifies thread-safe writes using multiple simultaneous threads.
4. **Benchmarking**: Measures sequential write and random read performance.

## 📐 Architecture
```text
+-----------------------+
|   Standard Library    |
+-----------------------+
           |
+-----------------------+
|        vfscore        | <--- VfsInode, VfsFile, VfsDentry Traits
+-----------------------+
           |
   +-------+-------+-------+
   |       |       |       |
+-------+ +-------+ +-------+ +-----------------+
| RamFs | | DevFs | | DynFs | |    dbfs-vfs     |
+-------+ +-------+ +-------+ +-----------------+
                                       |
                                +----------------+
                                |     dbfs2      |
                                +----------------+
                                       |
                                +----------------+
                                |     jammdb     |
                                +----------------+
```

## 📊 Feature Highlights
- **Persistence**: Files created in DBFS survive OS reboots (as long as the `.db` file is preserved).
- **Concurrency**: High-performance multi-threaded I/O support.
- **Extensibility**: Easily add new filesystem types by implementing `VfsInode` and `VfsFile`.

## 📚 Usage Example
```rust
let dbfs = dbfs_vfs::DBFSFs::new("my_storage", MyProvider);

// Mount to the root filesystem
let root_path = VfsPath::new(ramfs_root, ramfs_root);
root_path.join("mnt/db")?.mount(dbfs.mount(0, "/mnt/db", None, &[])?, 0)?;

// Standard file operations
let file = root_path.join("mnt/db/data.txt")?.open(Some(VfsInodeMode::FILE))?;
file.inode()?.write_at(0, b"Hello VFS!")?;
```

## 🔗 Reference
- [Linux Virtual File System Overview](https://docs.kernel.org/filesystems/vfs.html)
- [ArceOS axfs_vfs](https://github.com/rcore-os/arceos/tree/main/crates/axfs_vfs)
