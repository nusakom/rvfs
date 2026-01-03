//! DBFS-VFS: DBFS filesystem adapter for RVFS
//!
//! This module provides a VFS adapter for DBFS (Database Filesystem),
//! allowing it to be used within the RVFS framework.

use alloc::string::String;
use vfscore::utils::VfsTimeSpec;

// Re-export the adapter for convenience
pub use adapter::DbfsAdapter;

mod adapter;

/// DBFS filesystem provider (for future extensibility)
///
/// This trait defines the interface that DBFS requires from the runtime environment.
/// Currently, DBFS is self-contained and doesn't require external providers,
/// but this trait allows for future extensions.
pub trait DbfsProvider: Send + Sync {
    /// Get the current time (optional, DBFS has its own time handling)
    fn current_time(&self) -> VfsTimeSpec;
}

/// Default DBFS provider implementation
///
/// This implementation uses the system time when available.
#[derive(Debug, Clone)]
pub struct DefaultDbfsProvider;

impl DbfsProvider for DefaultDbfsProvider {
    fn current_time(&self) -> VfsTimeSpec {
        // Use a default timestamp
        // In a real implementation, this would get the actual system time
        VfsTimeSpec::default()
    }
}

#[cfg(test)]
mod tests;

// Dummy type for compilation without the dbfs feature
#[cfg(not(feature = "dbfs"))]
pub struct DbfsAdapter {
    db_path: String,
}

#[cfg(not(feature = "dbfs"))]
impl DbfsAdapter {
    pub fn new(db_path: String) -> Self {
        Self { db_path }
    }

    pub fn db_path(&self) -> &str {
        &self.db_path
    }

    pub fn fs_name(&self) -> String {
        "dbfs".to_string()
    }

    pub fn fs_flag(&self) -> vfscore::fstype::FileSystemFlags {
        vfscore::fstype::FileSystemFlags::REQUIRES_DEV
    }
}
