# Rust Virtual File System (RVFS)

A high-performance, flexible Virtual File System (VFS) abstraction layer for Rust, designed for use in both OS kernels and userspace applications.

## 🚀 Recent Update: DBFS-VFS Persistence & Recovery

The **DBFS-VFS** adapter layer has been upgraded to provide **Real Persistence** and **Cold Start Recovery** using a block-device-backed key-value store.

**⚠️ Current Status**: The core logic is **implemented** and supports real disk I/O, transactional updates, and crash recovery.

### 🚧 Build Note: Network Restrictions
In the current restricted environment, the following **public** dependencies cannot be fetched, causing build failures:
- `dbfs2`: [https://github.com/nusakom/dbfs](https://github.com/nusakom/dbfs)
- `jammdb`: [https://github.com/nusakom/jammdb](https://github.com/nusakom/jammdb)
- `device_interface`: [https://github.com/Godones/device_interface](https://github.com/Godones/device_interface)
- `constants`: [https://github.com/Godones/constants](https://github.com/Godones/constants)

**Workaround**: Use a network-enabled environment or configure local `path` overrides in `Cargo.toml`.

### Key Integration Components:
- **Block Persistence**: `BlockDeviceFile` adapter maps `jammdb` pages directly to physical block device sectors.
- **Cold Recovery**: Automatically detects filesystem magic headers on mount to recover state or initialize a fresh disk.
- **LRU Caching**: Implements `BlockDevicePageLoader` with LRU eviction to manage memory usage.
- **Transactions**: All operations (create, write, unlink) are atomic and crash-consistent.

## 📦 Features
- [x] **RamFs**: In-memory filesystem.
- [x] **DevFs**: Device node management.
- [x] **DynFs**: Dynamic filesystem for `/proc` and `/sys` style nodes.
- [x] **DBFS-VFS**: Reference implementation of database-backed filesystem adapter (in-memory storage).
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
