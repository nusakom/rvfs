//! # DBFS-VFS: Database-backed Filesystem VFS Adapter
//!
//! This crate provides a VFS (Virtual File System) adapter layer for database-backed filesystems.
//!
//! ## Current Implementation Status
//!
//! **⚠️ IMPORTANT**: This is currently a **reference implementation** using in-memory storage.
//!
//! - **What it is**: A VFS interface adapter demonstrating how to integrate a database-backed
//!   filesystem with the `vfscore` traits.
//! - **What it is NOT**: A production-ready persistent filesystem. The current implementation
//!   uses `BTreeMap` for in-memory storage and does **not** persist data to disk.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │   VFS Core Traits (vfscore)        │
//! │  (VfsInode, VfsFile, VfsDentry)    │
//! └─────────────────┬───────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────┐
//! │      DBFS-VFS Adapter Layer         │
//! │  (DBFSInodeAdapter, DBFSDentry)     │
//! └─────────────────┬───────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────┐
//! │   Storage Backend (pluggable)       │
//! │  Current: BTreeMap (in-memory)      │
//! │  Future: Real DBFS with persistence │
//! └─────────────────────────────────────┘
//! ```
//!
//! ## Future Work
//!
//! - [ ] Replace in-memory storage with actual DBFS backend
//! - [x] Add `.` and `..` entries to `readdir` for POSIX compliance
//! - [ ] Integrate with block device interface for persistence
//! - [ ] Add proper error handling and logging levels
//!
//! ## Usage
//!
//! ```rust,ignore
//! use dbfs_vfs::{DBFSFs, DBFSProvider};
//!
//! #[derive(Clone)]
//! struct MyProvider;
//!
//! impl DBFSProvider for MyProvider {
//!     fn current_time(&self) -> VfsTimeSpec {
//!         VfsTimeSpec::new(0, 0)
//!     }
//! }
//!
//! let dbfs = DBFSFs::<_, spin::Mutex<()>>::new("my_db", MyProvider);
//! let root = dbfs.mount(0, "/", None, &[]).unwrap();
//! ```

#![cfg_attr(not(test), no_std)]
#![feature(trait_alias)]

extern crate alloc;

use alloc::{
    string::{String, ToString},
    sync::{Arc, Weak},
    format,
    collections::BTreeMap,
    vec,
    vec::Vec,
};

use log::info;
use vfscore::{
    dentry::VfsDentry,
    error::VfsError,
    file::VfsFile,
    fstype::{FileSystemFlags, VfsFsType, VfsMountPoint},
    inode::VfsInode,
    superblock::VfsSuperBlock,
    utils::{VfsFileStat, VfsNodePerm, VfsNodeType, VfsTimeSpec, VfsDirEntry},
    VfsResult,
};

use lock_api::Mutex;
use core::sync::atomic::{AtomicUsize, Ordering};

pub trait VfsRawMutex = lock_api::RawMutex + Send + Sync;

/// Temporary in-memory storage backend using BTreeMap.
///
/// **NOTE**: This is a reference implementation for testing and demonstration purposes.
/// In a production system, this should be replaced with a proper persistent storage
/// backend (e.g., actual DBFS with block device integration).
///
/// The storage uses a simple key-value model:
/// - `i:{ino}` -> inode metadata
/// - `f:{ino}:0` -> file data (chunk 0)
/// - `d:{ino}` -> directory entries
type Storage = BTreeMap<Vec<u8>, Vec<u8>>;

static INODE_COUNTER: AtomicUsize = AtomicUsize::new(2);

// DBFS的dentry实现
pub struct DBFSDentry<R: VfsRawMutex> {
    inner: Mutex<R, DBFSDentryInner<R>>,
}

struct DBFSDentryInner<R: VfsRawMutex> {
    parent: Weak<dyn VfsDentry>,
    inode: Arc<dyn VfsInode>,
    name: String,
    mnt: Option<VfsMountPoint>,
    children: Option<BTreeMap<String, Arc<DBFSDentry<R>>>>,
}

impl<R: VfsRawMutex + 'static> DBFSDentry<R> {
    pub fn root(inode: Arc<dyn VfsInode>, parent: Weak<dyn VfsDentry>) -> Self {
        Self {
            inner: Mutex::new(DBFSDentryInner {
                parent,
                inode,
                name: "/".to_string(),
                mnt: None,
                children: Some(BTreeMap::new()),
            }),
        }
    }
}

impl<R: VfsRawMutex + 'static> VfsDentry for DBFSDentry<R> {
    fn name(&self) -> String {
        self.inner.lock().name.clone()
    }

    fn to_mount_point(
        self: Arc<Self>,
        sub_fs_root: Arc<dyn VfsDentry>,
        mount_flag: u32,
    ) -> VfsResult<()> {
        let point = self.clone() as Arc<dyn VfsDentry>;
        let mnt = VfsMountPoint {
            root: sub_fs_root,
            mount_point: Arc::downgrade(&point),
            mnt_flags: mount_flag,
        };
        if let Ok(p) = point.downcast_arc::<DBFSDentry<R>>() {
            let mut inner = p.inner.lock();
            inner.mnt = Some(mnt);
            Ok(())
        } else {
             Err(VfsError::Invalid)
        }
    }

    fn inode(&self) -> VfsResult<Arc<dyn VfsInode>> {
        Ok(self.inner.lock().inode.clone())
    }

    fn mount_point(&self) -> Option<VfsMountPoint> {
        self.inner.lock().mnt.clone()
    }

    fn clear_mount_point(&self) {
        self.inner.lock().mnt = None;
    }

    fn find(&self, path: &str) -> Option<Arc<dyn VfsDentry>> {
        let inner = self.inner.lock();
        inner.children.as_ref().and_then(|c| {
            c.get(path).map(|item| item.clone() as Arc<dyn VfsDentry>)
        })
    }

    fn insert(
        self: Arc<Self>,
        name: &str,
        child: Arc<dyn VfsInode>,
    ) -> VfsResult<Arc<dyn VfsDentry>> {
        let inode_type = child.inode_type();
        let child_dentry = Arc::new(DBFSDentry {
            inner: Mutex::new(DBFSDentryInner {
                parent: Arc::downgrade(&(self.clone() as Arc<dyn VfsDentry>)),
                inode: child,
                name: name.to_string(),
                mnt: None,
                children: match inode_type {
                    VfsNodeType::Dir => Some(BTreeMap::new()),
                    _ => None,
                },
            }),
        });
        let mut inner = self.inner.lock();
        if inner.children.is_none() {
            inner.children = Some(BTreeMap::new());
        }
        inner
            .children
            .as_mut()
            .unwrap()
            .insert(name.to_string(), child_dentry.clone())
            .map_or(Ok(child_dentry as Arc<dyn VfsDentry>), |_| Err(VfsError::EExist))
    }

    fn remove(&self, name: &str) -> Option<Arc<dyn VfsDentry>> {
        let mut inner = self.inner.lock();
        inner
            .children
            .as_mut()
            .and_then(|c| c.remove(name))
            .map(|x| x as Arc<dyn VfsDentry>)
    }

    fn parent(&self) -> Option<Arc<dyn VfsDentry>> {
        self.inner.lock().parent.upgrade()
    }

    fn set_parent(&self, parent: &Arc<dyn VfsDentry>) {
        let mut inner = self.inner.lock();
        inner.parent = Arc::downgrade(parent);
    }
}

pub trait DBFSProvider: Send + Sync + Clone {
    fn current_time(&self) -> VfsTimeSpec;
}

pub struct DBFSFs<T: Send + Sync, R: VfsRawMutex> {
    pub provider: T,
    storage: Arc<Mutex<R, Storage>>,
}

impl<T: DBFSProvider + 'static, R: VfsRawMutex + 'static> DBFSFs<T, R> {
    pub fn new(_db_name: &str, provider: T) -> Arc<Self> {
        let storage = Arc::new(Mutex::new(BTreeMap::new()));
        
        // Initialize root inode
        {
            let mut store = storage.lock();
            let root_key = b"i:1".to_vec();
            let root_data = vec![1u8; 8];
            store.insert(root_key, root_data);
        }
        
        Arc::new(Self { provider, storage })
    }
}

impl<T: DBFSProvider + 'static, R: VfsRawMutex + 'static> VfsFsType for DBFSFs<T, R> {
    fn mount(
        self: Arc<Self>,
        _flags: u32,
        _ab_mnt: &str,
        _dev: Option<Arc<dyn VfsInode>>,
        _data: &[u8],
    ) -> VfsResult<Arc<dyn VfsDentry>> {
        info!("Mounting DBFS via VFS adapter");
        
        let root_inode = Arc::new(DBFSInodeAdapter::new(1, self.storage.clone()));
        let parent = Weak::<DBFSDentry<R>>::new();
        let root_dentry = Arc::new(DBFSDentry::<R>::root(root_inode, parent));
        Ok(root_dentry as Arc<dyn VfsDentry>)
    }

    fn kill_sb(&self, _sb: Arc<dyn VfsSuperBlock>) -> VfsResult<()> {
        Ok(())
    }

    fn fs_flag(&self) -> FileSystemFlags {
        FileSystemFlags::empty()
    }

    fn fs_name(&self) -> String {
        "dbfs".to_string()
    }
}

pub struct DBFSInodeAdapter<R: VfsRawMutex> {
    ino: usize,
    storage: Arc<Mutex<R, Storage>>,
}

impl<R: VfsRawMutex> DBFSInodeAdapter<R> {
    pub fn new(ino: usize, storage: Arc<Mutex<R, Storage>>) -> Self {
        Self { ino, storage }
    }
}

impl<R: VfsRawMutex + 'static> VfsFile for DBFSInodeAdapter<R> {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let store = self.storage.lock();
        let key = format!("f:{}:0", self.ino).into_bytes();
        if let Some(data) = store.get(&key) {
            let offset = offset as usize;
            if offset >= data.len() {
                return Ok(0);
            }
            let read_len = core::cmp::min(buf.len(), data.len() - offset);
            buf[..read_len].copy_from_slice(&data[offset..offset+read_len]);
            Ok(read_len)
        } else {
            Ok(0)
        }
    }

    fn write_at(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        let mut store = self.storage.lock();
        let key = format!("f:{}:0", self.ino).into_bytes();
        let mut data = store.get(&key).cloned().unwrap_or_else(|| Vec::new());
        
        let offset = offset as usize;
        let required_len = offset + buf.len();
        if required_len > data.len() {
            data.resize(required_len, 0);
        }
        data[offset..required_len].copy_from_slice(buf);
        store.insert(key, data);
        
        Ok(buf.len())
    }

    fn readdir(&self, index: usize) -> VfsResult<Option<VfsDirEntry>> {
        // POSIX compliance: Return . and .. entries first
        match index {
            0 => {
                // Return "." entry pointing to self
                return Ok(Some(VfsDirEntry {
                    ino: self.ino as u64,
                    ty: VfsNodeType::Dir,
                    name: String::from("."),
                }));
            }
            1 => {
                // Return ".." entry
                // Note: In a real implementation, we'd track parent inode
                // For now, root's parent is itself, others point to root (ino 1)
                let parent_ino = if self.ino == 1 { 1 } else { 1 };
                return Ok(Some(VfsDirEntry {
                    ino: parent_ino as u64,
                    ty: VfsNodeType::Dir,
                    name: String::from(".."),
                }));
            }
            _ => {
                // Adjust index to account for . and .. entries
                let actual_index = index - 2;
                
                let store = self.storage.lock();
                let key = format!("d:{}", self.ino).into_bytes();
                if let Some(data) = store.get(&key) {
                    let mut offset = 0;
                    let mut current_index = 0;
                    
                    while offset < data.len() {
                        if offset + 13 > data.len() {
                            break;
                        }
                        
                        if current_index == actual_index {
                            let ino = u64::from_le_bytes(data[offset..offset+8].try_into().unwrap());
                            let ty_byte = data[offset+8];
                            let name_len = u32::from_le_bytes(data[offset+9..offset+13].try_into().unwrap()) as usize;
                            
                            if offset + 13 + name_len > data.len() {
                                break;
                            }
                            
                            let name = String::from_utf8(data[offset+13..offset+13+name_len].to_vec())
                                .unwrap_or_else(|_| String::from("?"));
                            
                            let ty = if ty_byte == 0 { VfsNodeType::Dir } else { VfsNodeType::File };
                            
                            return Ok(Some(VfsDirEntry { ino, ty, name }));
                        }
                        
                        let name_len = u32::from_le_bytes(data[offset+9..offset+13].try_into().unwrap()) as usize;
                        offset += 13 + name_len;
                        current_index += 1;
                    }
                }
                Ok(None)
            }
        }
    }
}

impl<R: VfsRawMutex + 'static> VfsInode for DBFSInodeAdapter<R> {
    fn node_perm(&self) -> VfsNodePerm {
        VfsNodePerm::from_bits_truncate(0o777)
    }

    fn create(
        &self,
        name: &str,
        ty: VfsNodeType,
        _perm: VfsNodePerm,
        _rdev: Option<u64>,
    ) -> VfsResult<Arc<dyn VfsInode>> {
        let mut store = self.storage.lock();
        let ino = INODE_COUNTER.fetch_add(1, Ordering::SeqCst);
        
        // Store inode metadata
        let inode_key = format!("i:{}", ino).into_bytes();
        let inode_data = vec![ino as u8; 8];
        store.insert(inode_key, inode_data);
        
        // Add to parent directory
        let dir_key = format!("d:{}", self.ino).into_bytes();
        let mut entries = store.get(&dir_key).cloned().unwrap_or_else(|| Vec::new());
        
        // Append new entry: ino (8) + type (1) + name_len (4) + name
        entries.extend_from_slice(&(ino as u64).to_le_bytes());
        entries.push(if ty == VfsNodeType::Dir { 0 } else { 1 });
        entries.extend_from_slice(&(name.len() as u32).to_le_bytes());
        entries.extend_from_slice(name.as_bytes());
        
        store.insert(dir_key, entries);
        
        Ok(Arc::new(DBFSInodeAdapter::new(ino, self.storage.clone())) as Arc<dyn VfsInode>)
    }

    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn VfsInode>> {
        let store = self.storage.lock();
        let key = format!("d:{}", self.ino).into_bytes();
        if let Some(data) = store.get(&key) {
            let mut offset = 0;
            
            while offset < data.len() {
                if offset + 13 > data.len() {
                    break;
                }
                
                let ino = u64::from_le_bytes(data[offset..offset+8].try_into().unwrap()) as usize;
                let name_len = u32::from_le_bytes(data[offset+9..offset+13].try_into().unwrap()) as usize;
                
                if offset + 13 + name_len > data.len() {
                    break;
                }
                
                let entry_name = String::from_utf8(data[offset+13..offset+13+name_len].to_vec())
                    .unwrap_or_else(|_| String::from("?"));
                
                if entry_name == name {
                    return Ok(Arc::new(DBFSInodeAdapter::new(ino, self.storage.clone())) as Arc<dyn VfsInode>);
                }
                
                offset += 13 + name_len;
            }
        }
        Err(VfsError::NoEntry)
    }

    fn get_attr(&self) -> VfsResult<VfsFileStat> {
        let mut stat = VfsFileStat::default();
        stat.st_ino = self.ino as u64;
        stat.st_mode = 0o777;
        stat.st_nlink = 1;
        
        // Try to get file size
        let store = self.storage.lock();
        let key = format!("f:{}:0", self.ino).into_bytes();
        if let Some(data) = store.get(&key) {
            stat.st_size = data.len() as u64;
        }
        
        Ok(stat)
    }

    fn inode_type(&self) -> VfsNodeType {
        // Check if this inode has directory entries
        let store = self.storage.lock();
        let key = format!("d:{}", self.ino).into_bytes();
        if store.get(&key).is_some() || self.ino == 1 {
            VfsNodeType::Dir
        } else {
            VfsNodeType::File
        }
    }

    fn truncate(&self, len: u64) -> VfsResult<()> {
        let mut store = self.storage.lock();
        let key = format!("f:{}:0", self.ino).into_bytes();
        let mut data = store.get(&key).cloned().unwrap_or_else(|| Vec::new());
        data.resize(len as usize, 0);
        store.insert(key, data);
        Ok(())
    }

    fn readlink(&self, _buf: &mut [u8]) -> VfsResult<usize> {
        Err(VfsError::NoSys)
    }

    fn symlink(&self, _name: &str, _target: &str) -> VfsResult<Arc<dyn VfsInode>> {
        Err(VfsError::NoSys)
    }

    fn unlink(&self, name: &str) -> VfsResult<()> {
        let mut store = self.storage.lock();
        let dir_key = format!("d:{}", self.ino).into_bytes();
        if let Some(data) = store.get(&dir_key).cloned() {
            let mut new_entries = Vec::new();
            let mut offset = 0;
            
            while offset < data.len() {
                if offset + 13 > data.len() {
                    break;
                }
                
                let name_len = u32::from_le_bytes(data[offset+9..offset+13].try_into().unwrap()) as usize;
                
                if offset + 13 + name_len > data.len() {
                    break;
                }
                
                let entry_name = String::from_utf8(data[offset+13..offset+13+name_len].to_vec())
                    .unwrap_or_else(|_| String::from("?"));
                
                if entry_name != name {
                    new_entries.extend_from_slice(&data[offset..offset+13+name_len]);
                }
                
                offset += 13 + name_len;
            }
            
            store.insert(dir_key, new_entries);
            Ok(())
        } else {
            Err(VfsError::NoEntry)
        }
    }

    fn rmdir(&self, name: &str) -> VfsResult<()> {
        self.unlink(name)
    }
}

#[cfg(test)]
mod tests;