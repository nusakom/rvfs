#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::{
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};

use log::info;
use alloc::collections::BTreeMap;
use vfscore::{
    dentry::VfsDentry,
    error::VfsError,
    file::VfsFile,
    fstype::{FileSystemFlags, VfsFsType, VfsMountPoint},
    inode::{InodeAttr, VfsInode},
    superblock::{SuperType, VfsSuperBlock},
    utils::{VfsFileStat, VfsFsStat, VfsNodePerm, VfsNodeType, VfsTimeSpec, VfsTime, VfsRenameFlag},
    VfsResult,
};

use alloc::{collections::BTreeMap, string::String, sync::{Arc, Weak}};
use dbfs2::{init_dbfs};
use jammdb::DB;
use lock_api::Mutex;
use downcast_rs::{impl_downcast, DowncastSync};

pub trait VfsRawMutex = lock_api::RawMutex + Send + Sync;

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

    pub fn new(inode: Arc<dyn VfsInode>, parent: Weak<dyn VfsDentry>, name: String) -> Self {
        Self {
            inner: Mutex::new(DBFSDentryInner {
                parent,
                inode,
                name,
                mnt: None,
                children: None,
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
        let point = self as Arc<dyn VfsDentry>;
        let mnt = VfsMountPoint {
            root: sub_fs_root.clone(),
            mount_point: Arc::downgrade(&point),
            mnt_flags: mount_flag,
        };
        let point = point
            .downcast_arc::<DBFSDentry<R>>()
            .map_err(|_| VfsError::Invalid)?;
        let mut inner = point.inner.lock();
        inner.mnt = Some(mnt);
        Ok(())
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
        let inode_type = inner.inode.inode_type();
        match inode_type {
            VfsNodeType::Dir => inner
                .children
                .as_ref()
                .unwrap()
                .get(path)
                .map(|item| item.clone() as Arc<dyn VfsDentry>),
            _ => None,
        }
    }

    fn insert(
        self: Arc<Self>,
        name: &str,
        child: Arc<dyn VfsInode>,
    ) -> VfsResult<Arc<dyn VfsDentry>> {
        let inode_type = child.inode_type();
        let child = Arc::new(DBFSDentry {
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
        self.inner
            .lock()
            .children
            .as_mut()
            .unwrap()
            .insert(name.to_string(), child.clone())
            .map_or(Ok(child), |_| Err(VfsError::EExist))
    }

    fn remove(&self, name: &str) -> Option<Arc<dyn VfsDentry>> {
        let mut inner = self.inner.lock();
        inner
            .children
            .as_mut()
            .unwrap()
            .remove(name)
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

impl_downcast!(sync VfsDentry);

pub trait DBFSProvider: Send + Sync + Clone {
    fn current_time(&self) -> VfsTimeSpec;
}

// 创建DBFS的vfscore适配器
pub struct DBFSFs<T: Send + Sync> {
    provider: T,
    db: Arc<DB>,
}

impl<T: DBFSProvider> DBFSFs<T> {
    pub fn new(provider: T, db: DB) -> Self {
        let db = Arc::new(db);
        init_dbfs(db.clone());
        Self { provider, db }
    }
}

// 简单的DBFS实现，用于测试
pub struct SimpleDBFSProvider;

impl Clone for SimpleDBFSProvider {
    fn clone(&self) -> Self {
        SimpleDBFSProvider
    }
}

impl Send for SimpleDBFSProvider {}
impl Sync for SimpleDBFSProvider {}

impl DBFSProvider for SimpleDBFSProvider {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::default()
    }
}

// 便捷的DBFS创建函数
pub type DBFS = DBFSFs<SimpleDBFSProvider>;

impl DBFS {
    pub fn new(db_name: &str) -> Arc<Self> {
        // 创建一个临时的jammdb数据库
        let db_path = format!("/tmp/{}.db", db_name);
        let db = jammdb::DB::open(&db_path).unwrap_or_else(|_| {
            // 如果打开失败，创建一个新的数据库
            jammdb::DB::create(&db_path).expect("Failed to create DBFS database")
        });
        Arc::new(DBFSFs::new(SimpleDBFSProvider, db))
    }
}

impl<T: DBFSProvider + 'static> VfsFsType for DBFSFs<T> {
    fn mount(
        self: Arc<Self>,
        _flags: u32,
        _ab_mnt: &str,
        _dev: Option<Arc<dyn VfsInode>>,
        _data: &[u8],
    ) -> VfsResult<Arc<dyn VfsDentry>> {
        info!("Mounting DBFS via VFS adapter");
        
        // 创建DBFS根目录的inode适配器
        let root_inode = Arc::new(DBFSInodeAdapter::new(1, self.db.clone()));
        
        // 创建根dentry
        let parent = Weak::<DBFSDentry<spin::Mutex<()>>>::new();
        let root_dentry = Arc::new(DBFSDentry::<spin::Mutex<()>>::root(root_inode, parent));
        
        Ok(root_dentry as Arc<dyn VfsDentry>)
    }

    fn kill_sb(&self, _sb: Arc<dyn VfsSuperBlock>) -> VfsResult<()> {
        info!("Unmounting DBFS");
        Ok(())
    }

    fn fs_flag(&self) -> FileSystemFlags {
        FileSystemFlags::empty()
    }

    fn fs_name(&self) -> String {
        "dbfs".to_string()
    }
}

// 实现DBFS的inode适配器，用于将DBFS的inode操作适配到vfscore接口
pub struct DBFSInodeAdapter {
    ino: usize,
    db: Arc<DB>,
}

impl DBFSInodeAdapter {
    pub fn new(ino: usize, db: Arc<DB>) -> Self {
        Self { ino, db }
    }
    
    // 将DBFS的属性转换为vfscore的VfsFileStat
    fn convert_attr_to_stat(&self, dbfs_attr: dbfs2::common::DbfsAttr) -> VfsFileStat {
        let mut stat = VfsFileStat::default();
        stat.st_ino = dbfs_attr.ino as u64;
        stat.st_size = dbfs_attr.size as u64;
        stat.st_mode = dbfs_attr.perm as u32;
        stat.st_nlink = dbfs_attr.nlink;
        stat.st_uid = dbfs_attr.uid;
        stat.st_gid = dbfs_attr.gid;
        stat.st_atime = dbfs_attr.atime.into();  // 需要实现From转换
        stat.st_mtime = dbfs_attr.mtime.into();
        stat.st_ctime = dbfs_attr.ctime.into();
        stat
    }
}

// 为DBFSInodeAdapter实现VfsFile trait（继承于VfsInode）
impl VfsFile for DBFSInodeAdapter {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        match dbfs2::dbfs_common_read(self.ino, buf, offset) {
            Ok(len) => Ok(len),
            Err(e) => Err(VfsError::from_dbfs_error(e)),
        }
    }

    fn write_at(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        match dbfs2::dbfs_common_write(self.ino, buf, offset) {
            Ok(len) => Ok(len),
            Err(e) => Err(VfsError::from_dbfs_error(e)),
        }
    }
}

// 为DBFSInodeAdapter实现VfsInode trait
impl VfsInode for DBFSInodeAdapter {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        // 目前返回错误，因为DBFSInodeAdapter不包含超级块信息
        // 在完整实现中需要存储超级块的引用
        Err(VfsError::NoSys)
    }

    fn node_perm(&self) -> VfsNodePerm {
        // 从DBFS获取权限信息
        match dbfs2::dbfs_common_attr(self.ino) {
            Ok(attr) => VfsNodePerm::from_bits_truncate(attr.perm as u32),
            Err(_) => VfsNodePerm::empty(),
        }
    }

    fn create(
        &self,
        name: &str,
        ty: VfsNodeType,
        perm: VfsNodePerm,
        _rdev: Option<u64>,
    ) -> VfsResult<Arc<dyn VfsInode>> {
        use dbfs2::common::{DbfsPermission, DbfsTimeSpec};
        
        // 将vfscore类型转换为DBFS类型
        let permission = match ty {
            VfsNodeType::Dir => DbfsPermission::S_IFDIR | DbfsPermission::from_bits_truncate(perm.bits()),
            VfsNodeType::File => DbfsPermission::S_IFREG | DbfsPermission::from_bits_truncate(perm.bits()),
            VfsNodeType::SymLink => DbfsPermission::S_IFLNK | DbfsPermission::from_bits_truncate(perm.bits()),
            VfsNodeType::CharDevice => DbfsPermission::S_IFCHR | DbfsPermission::from_bits_truncate(perm.bits()),
            VfsNodeType::BlockDevice => DbfsPermission::S_IFBLK | DbfsPermission::from_bits_truncate(perm.bits()),
            VfsNodeType::Fifo => DbfsPermission::S_IFIFO | DbfsPermission::from_bits_truncate(perm.bits()),
            VfsNodeType::Socket => DbfsPermission::S_IFSOCK | DbfsPermission::from_bits_truncate(perm.bits()),
            _ => DbfsPermission::S_IFREG | DbfsPermission::from_bits_truncate(perm.bits()),
        };
        
        let ctime = DbfsTimeSpec::default();
        match dbfs2::dbfs_common_create(self.ino, name, 0, 0, ctime, permission, None, None) {
            Ok(attr) => {
                // 创建新的inode适配器
                let new_inode = DBFSInodeAdapter::new(attr.ino, self.db.clone());
                Ok(Arc::new(new_inode))
            }
            Err(e) => Err(VfsError::from_dbfs_error(e)),
        }
    }

    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn VfsInode>> {
        match dbfs2::dbfs_common_lookup(self.ino, name) {
            Ok(attr) => {
                let inode = DBFSInodeAdapter::new(attr.ino, self.db.clone());
                Ok(Arc::new(inode))
            }
            Err(e) => Err(VfsError::from_dbfs_error(e)),
        }
    }

    fn get_attr(&self) -> VfsResult<VfsFileStat> {
        match dbfs2::dbfs_common_attr(self.ino) {
            Ok(attr) => {
                let stat = self.convert_attr_to_stat(attr);
                Ok(stat)
            }
            Err(e) => Err(VfsError::from_dbfs_error(e)),
        }
    }

    fn set_attr(&self, _attr: InodeAttr) -> VfsResult<()> {
        // 调用DBFS的设置属性接口
        Err(VfsError::NoSys)
    }

    fn inode_type(&self) -> VfsNodeType {
        // 从DBFS获取文件类型
        // 这里需要根据DBFS的内部类型进行转换
        use dbfs2::common::DbfsFileType;
        match dbfs2::dbfs_common_attr(self.ino) {
            Ok(attr) => {
                match DbfsFileType::from(attr.perm) {
                    DbfsFileType::Directory => VfsNodeType::Dir,
                    DbfsFileType::RegularFile => VfsNodeType::File,
                    DbfsFileType::Symlink => VfsNodeType::SymLink,
                    DbfsFileType::CharDevice => VfsNodeType::CharDevice,
                    DbfsFileType::BlockDevice => VfsNodeType::BlockDevice,
                    DbfsFileType::NamedPipe => VfsNodeType::Fifo,
                    DbfsFileType::Socket => VfsNodeType::Socket,
                }
            }
            Err(_) => VfsNodeType::Unknown,
        }
    }

    fn truncate(&self, len: u64) -> VfsResult<()> {
        use dbfs2::common::DbfsTimeSpec;
        let ctime = DbfsTimeSpec::default();
        match dbfs2::dbfs_common_truncate(0, 0, self.ino, ctime, len as usize) {
            Ok(_) => Ok(()),
            Err(e) => Err(VfsError::from_dbfs_error(e)),
        }
    }

    // 实现其他VfsInode方法...
    fn readlink(&self, buf: &mut [u8]) -> VfsResult<usize> {
        match dbfs2::dbfs_common_readlink(self.ino, buf) {
            Ok(len) => Ok(len),
            Err(e) => Err(VfsError::from_dbfs_error(e)),
        }
    }

    fn symlink(&self, name: &str, target: &str) -> VfsResult<Arc<dyn VfsInode>> {
        use dbfs2::common::{DbfsPermission, DbfsTimeSpec};
        let permission = DbfsPermission::S_IFLNK | DbfsPermission::from_bits_truncate(0o755);
        let ctime = DbfsTimeSpec::default();
        match dbfs2::dbfs_common_create(self.ino, name, 0, 0, ctime, permission, Some(target), None) {
            Ok(attr) => {
                let new_inode = DBFSInodeAdapter::new(attr.ino, self.db.clone());
                Ok(Arc::new(new_inode))
            }
            Err(e) => Err(VfsError::from_dbfs_error(e)),
        }
    }

    fn unlink(&self, name: &str) -> VfsResult<()> {
        use dbfs2::common::DbfsTimeSpec;
        let ctime = DbfsTimeSpec::default();
        match dbfs2::dbfs_common_unlink(0, 0, self.ino, name, None, ctime) {
            Ok(()) => Ok(()),
            Err(e) => Err(VfsError::from_dbfs_error(e)),
        }
    }

    fn rmdir(&self, name: &str) -> VfsResult<()> {
        use dbfs2::common::DbfsTimeSpec;
        let ctime = DbfsTimeSpec::default();
        match dbfs2::dbfs_common_rmdir(0, 0, self.ino, name, ctime) {
            Ok(()) => Ok(()),
            Err(e) => Err(VfsError::from_dbfs_error(e)),
        }
    }
    
    fn rename_to(
        &self,
        _old_name: &str,
        _new_parent: Arc<dyn VfsInode>,
        _new_name: &str,
        _flag: VfsRenameFlag,
    ) -> VfsResult<()> {
        Err(VfsError::NoSys)
    }
}

// 为VfsError添加从DbfsError的转换方法
impl VfsError {
    fn from_dbfs_error(dbfs_error: dbfs2::common::DbfsError) -> Self {
        match dbfs_error {
            dbfs2::common::DbfsError::PermissionDenied => VfsError::PermissionDenied,
            dbfs2::common::DbfsError::NotFound => VfsError::NoEntry,
            dbfs2::common::DbfsError::AccessError => VfsError::Access,
            dbfs2::common::DbfsError::FileExists => VfsError::EExist,
            dbfs2::common::DbfsError::InvalidArgument => VfsError::Invalid,
            dbfs2::common::DbfsError::NoSpace => VfsError::NoSpace,
            dbfs2::common::DbfsError::RangeError => VfsError::Invalid,
            dbfs2::common::DbfsError::NameTooLong => VfsError::NameTooLong,
            dbfs2::common::DbfsError::NoSys => VfsError::NoSys,
            dbfs2::common::DbfsError::NotEmpty => VfsError::NotEmpty,
            dbfs2::common::DbfsError::Io => VfsError::IoError,
            dbfs2::common::DbfsError::NotSupported => VfsError::NoSys,
            dbfs2::common::DbfsError::NoData => VfsError::NoEntry,
            dbfs2::common::DbfsError::Other => VfsError::Invalid,
        }
    }
}

// 实现从DbfsTimeSpec到VfsTimeSpec的转换
impl From<dbfs2::common::DbfsTimeSpec> for VfsTimeSpec {
    fn from(dbfs_time: dbfs2::common::DbfsTimeSpec) -> Self {
        VfsTimeSpec::new(dbfs_time.sec, dbfs_time.nsec as u64)
    }
}