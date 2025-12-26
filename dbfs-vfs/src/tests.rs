#![cfg(test)]

use super::*;

#[derive(Clone)]
struct TestProvider;

impl DBFSProvider for TestProvider {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
}

#[test]
fn test_dbfs_basic_operations() {
    let dbfs = DBFSFs::<_, spin::Mutex<()>>::new("test_basic", TestProvider);
    let root = dbfs.clone().mount(0, "/", None, &[]).unwrap();
    let root_inode = root.inode().unwrap();
    
    // Create a file
    let file_inode = root_inode.create("test.txt", VfsNodeType::File, VfsNodePerm::from_bits_truncate(0o666), None).unwrap();
    
    // Write data
    let data = b"Hello DBFS!";
    let written = file_inode.write_at(0, data).unwrap();
    assert_eq!(written, data.len());
    
    // Read data back
    let mut buf = [0u8; 64];
    let read = file_inode.read_at(0, &mut buf).unwrap();
    assert_eq!(read, data.len());
    assert_eq!(&buf[..read], data);
}

#[test]
fn test_dbfs_directory_operations() {
    let dbfs = DBFSFs::<_, spin::Mutex<()>>::new("test_dir", TestProvider);
    let root = dbfs.clone().mount(0, "/", None, &[]).unwrap();
    let root_inode = root.inode().unwrap();
    
    // Create directory
    let dir_inode = root_inode.create("testdir", VfsNodeType::Dir, VfsNodePerm::from_bits_truncate(0o755), None).unwrap();
    
    // Create file in directory
    let _file_inode = dir_inode.create("file.txt", VfsNodeType::File, VfsNodePerm::from_bits_truncate(0o666), None).unwrap();
    
    // Lookup file
    let found_inode = dir_inode.lookup("file.txt").unwrap();
    assert_eq!(found_inode.inode_type(), VfsNodeType::File);
}

#[test]
fn test_dbfs_readdir() {
    let dbfs = DBFSFs::<_, spin::Mutex<()>>::new("test_readdir", TestProvider);
    let root = dbfs.clone().mount(0, "/", None, &[]).unwrap();
    let root_inode = root.inode().unwrap();
    
    // Create multiple files
    root_inode.create("file1.txt", VfsNodeType::File, VfsNodePerm::from_bits_truncate(0o666), None).unwrap();
    root_inode.create("file2.txt", VfsNodeType::File, VfsNodePerm::from_bits_truncate(0o666), None).unwrap();
    root_inode.create("dir1", VfsNodeType::Dir, VfsNodePerm::from_bits_truncate(0o755), None).unwrap();
    
    // Read directory entries
    let mut entries = Vec::new();
    for i in 0..10 {
        if let Some(entry) = root_inode.readdir(i).unwrap() {
            entries.push(entry);
        } else {
            break;
        }
    }
    
    // Should have: . .. file1.txt file2.txt dir1 = 5 entries
    assert_eq!(entries.len(), 5);
    
    // Verify . and .. are first
    assert_eq!(entries[0].name, ".");
    assert_eq!(entries[1].name, "..");
    
    // Verify other entries exist (order may vary)
    let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"file1.txt"));
    assert!(names.contains(&"file2.txt"));
    assert!(names.contains(&"dir1"));
}

#[test]
fn test_dbfs_truncate() {
    let dbfs = DBFSFs::<_, spin::Mutex<()>>::new("test_truncate", TestProvider);
    let root = dbfs.clone().mount(0, "/", None, &[]).unwrap();
    let root_inode = root.inode().unwrap();
    
    let file_inode = root_inode.create("test.txt", VfsNodeType::File, VfsNodePerm::from_bits_truncate(0o666), None).unwrap();
    
    // Write data
    let data = b"Hello World!";
    file_inode.write_at(0, data).unwrap();
    
    // Truncate to smaller size
    file_inode.truncate(5).unwrap();
    
    // Read back
    let mut buf = [0u8; 64];
    let read = file_inode.read_at(0, &mut buf).unwrap();
    assert_eq!(read, 5);
    assert_eq!(&buf[..read], b"Hello");
}

#[test]
fn test_dbfs_unlink() {
    let dbfs = DBFSFs::<_, spin::Mutex<()>>::new("test_unlink", TestProvider);
    let root = dbfs.clone().mount(0, "/", None, &[]).unwrap();
    let root_inode = root.inode().unwrap();
    
    // Create file
    root_inode.create("test.txt", VfsNodeType::File, VfsNodePerm::from_bits_truncate(0o666), None).unwrap();
    
    // Verify it exists
    assert!(root_inode.lookup("test.txt").is_ok());
    
    // Unlink it
    root_inode.unlink("test.txt").unwrap();
    
    // Verify it's gone
    assert!(root_inode.lookup("test.txt").is_err());
}
