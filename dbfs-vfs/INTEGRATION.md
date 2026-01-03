# DBFS-VFS Integration Guide

本文档说明如何将 DBFS 文件系统集成到 RVFS 框架中。

## 架构概述

```
┌─────────────────────────────────────────────────────────┐
│                     RVFS Application                     │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │     Filesystem Registry (HashMap<String, FsType>) │  │
│  │                                                   │  │
│  │  "ramfs"  → RamFs                                │  │
│  │  "devfs"  → DevFs                                │  │
│  │  "dbfs"   → DbFsAdapter  ← DBFS 集成点           │  │
│  │  "ext4"   → ExtFs                                │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│                    vfscore (VFS Layer)                   │
│                                                          │
│  • VfsFsType trait                                      │
│  • VfsSuperBlock trait                                  │
│  • VfsInode trait                                       │
│  • VfsDentry trait                                      │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│                    dbfs-vfs (适配器)                     │
│                                                          │
│  pub struct DbFsAdapter {                               │
│      db_path: String,                                   │
│      inner: Arc<dbfs2::rvfs2::DbfsFsType>,             │
│  }                                                       │
│                                                          │
│  impl VfsFsType for DbFsAdapter                         │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│              dbfs2 (独立的 Git 仓库)                     │
│                                                          │
│  • src/rvfs2/  ← RVFS2 适配代码                          │
│  • DbfsFsType                                           │
│  • DbfsSuperBlock                                       │
│  • DbfsInode                                            │
│  • DbfsDentry                                           │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│              jammdb (数据库引擎)                         │
└─────────────────────────────────────────────────────────┘
```

## 集成步骤

### 1. 在 RVFS workspace 中添加 dbfs-vfs

已自动完成：`dbfs-vfs` 已添加到 RVFS workspace 的成员列表中。

### 2. 在应用中注册 DBFS

```rust
use std::sync::Arc;
use vfscore::fstype::VfsFsType;
use dbfs_vfs::DbFsAdapter;

// 创建 DBFS 适配器实例
let dbfs = Arc::new(DbFsAdapter::new("/path/to/database.db".to_string()));

// 注册到文件系统注册表
FILESYSTEM_REGISTRY.lock().insert("dbfs".to_string(), dbfs);
```

### 3. 挂载 DBFS

```rust
// 获取文件系统类型
let dbfs_fs = get_filesystem("dbfs").unwrap();

// 挂载到指定路径
let root_dentry = dbfs_fs.mount(
    0,              // flags: 挂载标志
    "/mnt/dbfs",    // ab_mnt: 绝对挂载路径
    None,           // dev: 设备 inode (可选)
    &[],            // data: 额外数据 (可选)
)?;

// 现在可以通过 VFS 层访问文件系统
```

### 4. 使用文件系统

挂载后，所有文件操作都通过标准的 VFS API 进行：

```rust
use vfscore::inode::VfsInode;

// 获取根 inode
let root_inode = root_dentry.inode()?;

// 创建文件
let file_inode = root_inode.create(
    "test.txt",
    vfscore::utils::VfsNodeType::File,
    0o755,
)?;

// 写入数据
let mut file = file_inode.open(0)?;
file.write_at(b"Hello, DBFS!", 0)?;

// 读取数据
let mut buffer = [0u8; 1024];
let bytes_read = file.read_at(0, &mut buffer)?;
```

## 多实例支持

DBFS 支持同时挂载多个实例，每个实例使用不同的数据库文件：

```rust
// 数据库专用实例
let dbfs_data = Arc::new(DbFsAdapter::new(
    "/var/lib/dbfs/data.db".to_string()
));

// 日志专用实例
let dbfs_log = Arc::new(DbFsAdapter::new(
    "/var/lib/dbfs/log.db".to_string()
));

// 缓存专用实例
let dbfs_cache = Arc::new(DbFsAdapter::new(
    "/var/lib/dbfs/cache.db".to_string()
));

// 注册为不同的文件系统类型
registry.insert("dbfs-data".to_string(), dbfs_data);
registry.insert("dbfs-log".to_string(), dbfs_log);
registry.insert("dbfs-cache".to_string(), dbfs_cache);

// 分别挂载到不同的挂载点
mount("dbfs-data", "/mnt/data")?;
mount("dbfs-log", "/mnt/log")?;
mount("dbfs-cache", "/mnt/cache")?;
```

## 构建和运行

### 编译 dbfs-vfs

```bash
cd /path/to/rvfs/dbfs-vfs

# 编译（不包含 dbfs2，用于类型检查）
cargo build

# 编译并包含 dbfs2 依赖
cargo build --features dbfs
```

### 在 demo 中使用

修改 `rvfs/demo/Cargo.toml`：

```toml
[dependencies]
dbfs-vfs = { path = "../dbfs-vfs", features = ["dbfs"] }
```

修改 `rvfs/demo/src/main.rs`：

```rust
use dbfs_vfs::DbFsAdapter;

fn register_all_fs() {
    // ... 其他文件系统 ...

    let dbfs = Arc::new(DbFsAdapter::new("/tmp/dbfs.db".to_string()));
    FS.lock().insert("dbfs".to_string(), dbfs);
}
```

## 运行时示例

```bash
# 启动 demo
cd rvfs/demo
cargo run --features dbfs

# 在另一个终端，挂载 DBFS
mkdir -p /mnt/dbfs
mount -t dbfs /dev/db0 /mnt/dbfs

# 使用文件系统
echo "Hello, DBFS!" > /mnt/dbfs/test.txt
cat /mnt/dbfs/test.txt
```

## 依赖关系

```
dbfs-vfs
  ├─→ vfscore (workspace 本地依赖)
  └─→ dbfs2 (Git 依赖: ssh://git@github.com/nusakom/dbfs.git)
       ├─→ vfscore (Git 依赖: https://github.com/os-module/rvfs.git)
       └─→ jammdb (Git 依赖: ssh://git@github.com/nusakom/jammdb.git)
```

## 注意事项

1. **数据库文件必须存在**：挂载前需要确保数据库文件已初始化
2. **Git 依赖权限**：确保有访问 `nusakom/dbfs` 和 `nusakom/jammdb` 的权限
3. **feature flag**：使用 `--features dbfs` 启用 DBFS 集成
4. **线程安全**：DbFsAdapter 实现了 Send + Sync，可以在线程间共享
5. **错误处理**：所有数据库错误都会转换为 VfsError::IoError

## 与其他文件系统的对比

| 文件系统 | 存储介质 | 用途 | 特点 |
|---------|---------|------|------|
| ramfs  | 内存     | 临时文件 | 快速，数据不持久 |
| devfs  | 虚拟     | 设备管理 | 动态生成设备节点 |
| fat-vfs| 块设备   | 通用存储 | 广泛兼容性 |
| lwext4-vfs | 块设备 | 高性能 | 日志文件系统 |
| **dbfs-vfs** | **数据库** | **结构化数据** | **事务支持，元数据丰富** |

## 下一步

1. 在 demo 应用中测试集成
2. 实现完整的文件操作测试用例
3. 添加性能基准测试
4. 编写用户文档
5. 考虑添加 DBFS 特有的功能（如查询、索引等）

## 参考

- RVFS 仓库: https://github.com/os-module/rvfs
- DBFS2 仓库: https://github.com/nusakom/dbfs
- vfscore 文档: https://docs.rs/vfscore
