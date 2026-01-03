//! DBFS adapter implementation
//!
//! This module provides the core adapter that bridges DBFS and RVFS.

use alloc::{string::String, sync::Arc};
use log::info;
use vfscore::{
    dentry::VfsDentry,
    error::VfsError,
    fstype::{FileSystemFlags, VfsFsType},
    inode::VfsInode,
    VfsResult,
};

#[cfg(feature = "dbfs")]
use dbfs2::rvfs2::DbfsFsType;

/// DBFS filesystem type for RVFS integration
#[cfg(feature = "dbfs")]
pub struct DbfsAdapter {
    /// Database path
    db_path: String,
    /// Inner DBFS filesystem type
    inner: Arc<DbfsFsType>,
}

#[cfg(feature = "dbfs")]
impl DbfsAdapter {
    /// Create a new DBFS adapter
    ///
    /// # Arguments
    /// * `db_path` - Path to the DBFS database file
    pub fn new(db_path: String) -> Self {
        info!("Creating DBFS adapter for database: {}", db_path);
        let inner = Arc::new(DbfsFsType::new(db_path.clone()));
        Self { db_path, inner }
    }

    /// Get the database path
    pub fn db_path(&self) -> &str {
        &self.db_path
    }
}

#[cfg(feature = "dbfs")]
impl VfsFsType for DbfsAdapter {
    fn mount(
        self: Arc<Self>,
        flags: u32,
        ab_mnt: &str,
        dev: Option<Arc<dyn VfsInode>>,
        data: &[u8],
    ) -> VfsResult<Arc<dyn VfsDentry>> {
        info!(
            "Mounting DBFS at {} with database: {}",
            ab_mnt, self.db_path
        );

        // Delegate to the inner DbfsFsType
        self.inner.mount(flags, ab_mnt, dev, data)
    }

    fn kill_sb(&self, sb: Arc<dyn vfscore::superblock::VfsSuperBlock>) -> VfsResult<()> {
        info!("Unmounting DBFS from database: {}", self.db_path);
        self.inner.kill_sb(sb)
    }

    fn fs_flag(&self) -> FileSystemFlags {
        self.inner.fs_flag()
    }

    fn fs_name(&self) -> String {
        self.inner.fs_name()
    }
}
