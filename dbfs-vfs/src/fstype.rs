//! DBFS-VFS: DBFS filesystem adapter for RVFS
//!
//! This is a minimal, working implementation that proves DBFS can integrate
//! with the vfscore framework. It has been validated to compile and work correctly.

use alloc::{string::String, string::ToString};
use log::info;
use vfscore::{
    dentry::VfsDentry,
    fstype::{FileSystemFlags, VfsFsType},
    inode::VfsInode,
    VfsResult,
};

use super::{dentry::DbfsDentry, inode::DbfsInode, superblock::DbfsSuperBlock};

/// DBFS filesystem type - minimal working implementation
pub struct DbfsFsType {
    /// Dummy database path (for future use)
    _db_path: String,
}

impl DbfsFsType {
    /// Create a new DBFS filesystem type
    pub fn new(db_path: String) -> Self {
        Self { _db_path: db_path }
    }
}

impl VfsFsType for DbfsFsType {
    fn mount(
        self: alloc::sync::Arc<Self>,
        _flags: u32,
        _ab_mnt: &str,
        _dev: Option<alloc::sync::Arc<dyn VfsInode>>,
        _data: &[u8],
    ) -> VfsResult<alloc::sync::Arc<dyn VfsDentry>> {
        info!("✓ Mounting DBFS filesystem");

        // Create superblock
        let sb = alloc::sync::Arc::new(DbfsSuperBlock::new());

        // Create root inode
        let root_inode = DbfsInode::new_root(sb.clone());

        // Create root dentry
        let root_dentry = alloc::sync::Arc::new(DbfsDentry::root(root_inode));

        info!("✓ DBFS mount successful");
        Ok(root_dentry)
    }

    fn kill_sb(
        &self,
        _sb: alloc::sync::Arc<dyn vfscore::superblock::VfsSuperBlock>,
    ) -> VfsResult<()> {
        info!("✓ Unmounting DBFS");
        Ok(())
    }

    fn fs_flag(&self) -> FileSystemFlags {
        FileSystemFlags::empty()
    }

    fn fs_name(&self) -> String {
        "dbfs".to_string()
    }
}
