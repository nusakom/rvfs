//! DBFS-VFS Integration Example
//!
//! This example demonstrates how to integrate DBFS into an RVFS-based application
//! and register it alongside other filesystems.

use std::sync::{Arc, Mutex};
use vfscore::fstype::VfsFsType;

// Import DBFS adapter
#[cfg(feature = "dbfs")]
use dbfs_vfs::DbfsAdapter;

/// Filesystem registry
///
/// This pattern is commonly used in RVFS applications to manage
/// multiple filesystem types that can be mounted at runtime.
static FILESYSTEM_REGISTRY: Mutex<Option<std::collections::HashMap<String, Arc<dyn VfsFsType>>>> =
    Mutex::new(None);

/// Initialize the filesystem registry with all available filesystems
fn register_filesystems() {
    let mut registry = std::collections::HashMap::new();

    // Register DBFS
    #[cfg(feature = "dbfs")]
    {
        let dbfs = Arc::new(DbFsAdapter::new("/tmp/dbfs_database.db".to_string()));
        registry.insert("dbfs".to_string(), dbfs);
        println!("✓ Registered DBFS filesystem");
    }

    // Register other filesystems (examples)
    // let ramfs = Arc::new(ramfs::RamFs::new(ramfs::RamFsProviderImpl));
    // registry.insert("ramfs".to_string(), ramfs);

    // let devfs = Arc::new(devfs::DevFs::new(devfs::DevFsKernelProviderImpl));
    // registry.insert("devfs".to_string(), devfs);

    *FILESYSTEM_REGISTRY.lock().unwrap() = Some(registry);
}

/// Get a filesystem by name
fn get_filesystem(name: &str) -> Option<Arc<dyn VfsFsType>> {
    FILESYSTEM_REGISTRY
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|reg| reg.get(name).cloned())
}

/// Example: Mount DBFS at a specific path
fn example_mount_dbfs() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== DBFS Mount Example ===\n");

    // Get the DBFS filesystem type
    let dbfs_fs = get_filesystem("dbfs")
        .ok_or("DBFS filesystem not registered")?;

    // Mount DBFS at /mnt/dbfs
    let mount_point = "/mnt/dbfs";
    let flags = 0; // Mount flags (read-write, etc.)

    println!("Mounting DBFS at {}...", mount_point);

    match dbfs_fs.mount(flags, mount_point, None, &[]) {
        Ok(root_dentry) => {
            println!("✓ Successfully mounted DBFS");
            println!("  Root dentry: {}", root_dentry.name());
            println!("  Root inode type: {:?}", root_dentry.inode()?.inode_type());

            // The filesystem is now mounted and ready to use
            // You can now perform file operations through the VFS layer

            Ok(())
        }
        Err(e) => {
            eprintln!("✗ Failed to mount DBFS: {:?}", e);
            Err(Box::new(e))
        }
    }
}

/// Example: Mount multiple DBFS instances
fn example_multiple_databases() {
    println!("\n=== Multiple DBFS Instances Example ===\n");

    let mut registry = std::collections::HashMap::new();

    // Register multiple DBFS instances with different databases
    registry.insert(
        "dbfs-data".to_string(),
        Arc::new(DbFsAdapter::new("/var/lib/dbfs/data.db".to_string()))
            as Arc<dyn VfsFsType>,
    );

    registry.insert(
        "dbfs-log".to_string(),
        Arc::new(DbFsAdapter::new("/var/lib/dbfs/log.db".to_string()))
            as Arc<dyn VfsFsType>,
    );

    registry.insert(
        "dbfs-cache".to_string(),
        Arc::new(DbFsAdapter::new("/var/lib/dbfs/cache.db".to_string()))
            as Arc<dyn VfsFsType>,
    );

    println!("✓ Registered {} DBFS instances", registry.len());

    // Each instance can be mounted independently at different mount points
    // For example:
    // - dbfs-data → /mnt/data
    // - dbfs-log → /mnt/log
    // - dbfs-cache → /mnt/cache
}

fn main() {
    println!("╔══════════════════════════════════════════╗");
    println!("║   DBFS-VFS Integration Example          ║");
    println!("╚══════════════════════════════════════════╝\n");

    // Register all filesystems
    register_filesystems();

    // Example 1: Mount a single DBFS instance
    #[cfg(feature = "dbfs")]
    if let Err(e) = example_mount_dbfs() {
        eprintln!("Error: {}", e);
    }

    // Example 2: Multiple DBFS instances
    #[cfg(feature = "dbfs")]
    example_multiple_databases();

    println!("\n=== Integration Complete ===");
    println!("\nKey Points:");
    println!("  1. DBFS is registered like any other VFS filesystem");
    println!("  2. Multiple DBFS instances can coexist with different databases");
    println!("  3. Mount points are managed by the VFS layer");
    println!("  4. File operations go through the standard VFS API");
    println!("\nFor production use:");
    println!("  - Ensure database files exist before mounting");
    println!("  - Handle mount/unmount lifecycle properly");
    println!("  - Implement error handling for database operations");
}

// Dummy type for compilation without the dbfs feature
#[cfg(not(feature = "dbfs"))]
struct DbFsAdapter(String);

#[cfg(not(feature = "dbfs"))]
impl DbFsAdapter {
    fn new(path: String) -> Self {
        Self(path)
    }
}

#[cfg(not(feature = "dbfs"))]
impl VfsFsType for DbFsAdapter {
    fn mount(
        self: Arc<Self>,
        _flags: u32,
        _ab_mnt: &str,
        _dev: Option<Arc<dyn vfscore::inode::VfsInode>>,
        _data: &[u8],
    ) -> vfscore::VfsResult<Arc<dyn vfscore::dentry::VfsDentry>> {
        Err(vfscore::error::VfsError::NoSys)
    }

    fn kill_sb(&self, _sb: Arc<dyn vfscore::superblock::VfsSuperBlock>) -> vfscore::VfsResult<()> {
        Ok(())
    }

    fn fs_flag(&self) -> vfscore::fstype::FileSystemFlags {
        vfscore::fstype::FileSystemFlags::REQUIRES_DEV
    }

    fn fs_name(&self) -> String {
        "dbfs".to_string()
    }
}
