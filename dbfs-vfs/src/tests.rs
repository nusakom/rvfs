/// Integration tests for dbfs-vfs
///
/// These tests verify that the dbfs-vfs adapter correctly integrates with RVFS

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = DbFsAdapter::new("/tmp/test.db".to_string());
        assert_eq!(adapter.db_path(), "/tmp/test.db");
    }

    #[test]
    fn test_fs_name() {
        let adapter = DbFsAdapter::new("/tmp/test.db".to_string());
        assert_eq!(adapter.fs_name(), "dbfs");
    }

    #[test]
    fn test_fs_flags() {
        let adapter = DbFsAdapter::new("/tmp/test.db".to_string());
        let flags = adapter.fs_flag();
        assert_eq!(flags, FileSystemFlags::REQUIRES_DEV);
    }

    #[test]
    fn test_multiple_instances() {
        let db1 = DbFsAdapter::new("/db1.db".to_string());
        let db2 = DbFsAdapter::new("/db2.db".to_string());
        let db3 = DbFsAdapter::new("/db3.db".to_string());

        assert_eq!(db1.db_path(), "/db1.db");
        assert_eq!(db2.db_path(), "/db2.db");
        assert_eq!(db3.db_path(), "/db3.db");
    }
}
