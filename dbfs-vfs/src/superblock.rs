//! DBFS SuperBlock - Minimal working implementation

use alloc::sync::Arc;
use vfscore::{
    superblock::{SuperType, VfsSuperBlock},
    utils::VfsFsStat,
    VfsResult,
};

/// DBFS SuperBlock
pub struct DbfsSuperBlock {
    _block_size: u64,
}

impl DbfsSuperBlock {
    /// Create a new superblock
    pub fn new() -> Self {
        Self { _block_size: 4096 }
    }
}

impl VfsSuperBlock for DbfsSuperBlock {
    fn sync_fs(&self, _wait: bool) -> VfsResult<()> {
        Ok(())
    }

    fn stat_fs(&self) -> VfsResult<VfsFsStat> {
        Ok(VfsFsStat {
            f_bsize: 4096,
            f_frsize: 4096,
            f_blocks: 1024,
            f_bfree: 512,
            f_bavail: 512,
            f_files: 100,
            f_ffree: 50,
            f_favail: 50,
            f_fsid: 0x44424653, // "DBFS"
            f_flag: 0,
            f_namemax: 255,
            name: [0; 32],
        })
    }

    fn super_type(&self) -> SuperType {
        SuperType::Other
    }

    fn fs_type(&self) -> alloc::sync::Arc<dyn vfscore::fstype::VfsFsType> {
        alloc::sync::Arc::new(super::fstype::DbfsFsType::new(
            "/tmp/demo.db".to_string(),
        )) as alloc::sync::Arc<dyn vfscore::fstype::VfsFsType>
    }

    fn root_inode(&self) -> VfsResult<alloc::sync::Arc<dyn vfscore::inode::VfsInode>> {
        Ok(alloc::sync::Arc::new(super::inode::DbfsInode::new_root(
            alloc::sync::Arc::clone(self),
        )) as alloc::sync::Arc<dyn vfscore::inode::VfsInode>)
    }
}
