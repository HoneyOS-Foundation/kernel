#[cfg(test)]
mod filetable_tests {
    use crate::fstable::FsTable;

    #[test]
    fn file_creation() {
        let mut table = FsTable::new();

        let foo_dir_id = table.create_dir("foo").unwrap();
        let bar_file_id = table.create_file("foo/bar").unwrap();

        assert_eq!(Ok(foo_dir_id), table.get_dir("foo"));
        assert_eq!(Ok(bar_file_id), table.get_file("foo/bar"));
    }
}
