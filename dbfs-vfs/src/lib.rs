#![cfg_attr(not(test), no_std)]
extern crate alloc;

//! DBFS-VFS: DBFS filesystem adapter for RVFS

mod dentry;
mod fstype;
mod inode;
mod superblock;

pub use fstype::DbfsFsType;
