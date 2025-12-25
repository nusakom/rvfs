# VFS
This crate provides a virtual file system implementation, which can be used in the kernel or user space.

The virtual file system is a file system abstraction layer. The virtual file system is responsible for managing all file systems, and all file system types must be registered in the virtual file system.


## Features
- [x] RamFs
- [x] DevFs
- [x] DynFs(It can be used as procfs/sysfs)
- [x] VfsCore
- [x] ExtFs
- [x] FatFs
- [x] DBFS
- [ ] ...


## Demo
```bash
# run
RUST_LOG=info cargo run -p demo
```


## Usage
```
devfs = { git = "https://github.com/os-module/rvfs" }
ramfs = { git = "https://github.com/os-module/rvfs" }
dynfs = { git = "https://github.com/os-module/rvfs" }
fat-vfs = { git = "https://github.com/os-module/rvfs" }
lwext-vfs = { git = "https://github.com/os-module/rvfs" }
dbfs-vfs = { git = "https://github.com/os-module/rvfs" }
vfscore = { git = "https://github.com/os-module/rvfs" }
```
```rust
// create a fs_type
let ramfs = Arc::new(RamFs::<_, Mutex<()>>::new(RamFsProviderImpl));
let dbfs = Arc::new(DBFS::new("test_db"));

// create a fs instance
let root_dt = ramfs.i_mount(MountFlags::empty(), None, &[])?;
let dbfs_root = dbfs.i_mount(MountFlags::empty(), None, &[])?;

// get the inode
let root_inode = root_dt.inode()?;
let dbfs_inode = dbfs_root.inode()?;

// create a file
let f1 = root_inode.create(
    "f1.txt",
    VfsNodeType::File,
    VfsNodePerm::from_bits_truncate(0o666),
    None,
)?;

// create a file in DBFS
let db_file = dbfs_inode.create(
    "db_file.txt",
    VfsNodeType::File,
    VfsNodePerm::from_bits_truncate(0o666),
    None,
)?;
```



## DBFS Integration with VfsCore
DBFS (Database-based File System) has been successfully integrated with the VfsCore framework, providing a database-backed file system that supports standard POSIX file operations. The integration includes:

- **DBFSInodeAdapter**: Adapts DBFS internal operations to VfsCore's VfsInode trait
- **DBFSDentry**: Implements VfsDentry trait for directory entry management
- **VfsFile and VfsInode trait implementations**: Full support for read, write, create, lookup, and other file operations
- **Error handling**: Proper conversion between DBFS errors and VfsCore errors
- **Time specification conversion**: Support for converting between different time representations
- **Complete file system operations**: Support for file creation, directory operations, symlinks, and file attribute management

This integration allows DBFS to work seamlessly with other file systems in the rvfs ecosystem, without requiring any modifications to the VfsCore framework itself.

## Reference

[Overview of the Linux Virtual File System — The Linux Kernel documentation](https://docs.kernel.org/filesystems/vfs.html)

https://github.com/rcore-os/arceos/tree/main/crates/axfs_vfs

https://github.com/yfblock/ByteOS

