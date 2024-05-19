#[cfg(test)]
mod filetable_tests {
    use crate::fstable::FsTable;

    #[test]
    fn file_creation() {
        let mut table = FsTable::new();

        let foo_dir_id = table.create_dir("foo").unwrap();
        let bar_file_id = table.create_file("foo/bar").unwrap();
        let spam_file_id = table.create_file("spam.txt").unwrap();

        assert_eq!(foo_dir_id, table.get_directory_from_path("foo").unwrap());
        assert_eq!(bar_file_id, table.get_file_from_path("foo/bar").unwrap());
        assert_eq!(spam_file_id, table.get_file_from_path("spam.txt").unwrap());
    }

    #[test]
    fn children() {
        let mut table = FsTable::new();

        table.create_dir("foo").unwrap();
        let bar_id = table.create_dir("foo/bar").unwrap();
        table.create_dir("foo/bar/spam").unwrap();
        let eggs_id = table.create_file("foo/bar/spam/eggs").unwrap();

        assert_eq!(bar_id, table.get_directory_from_path("foo/bar").unwrap());
        assert_eq!(
            eggs_id,
            table.get_file_from_path("foo/bar/spam/eggs").unwrap()
        );
    }

    #[test]
    fn normalization() {
        let mut table = FsTable::new();

        table.create_dir("foo").unwrap();
        let bar_id = table.create_dir("foo/bar").unwrap();
        table.create_dir("foo/bar/spam").unwrap();
        table.create_file("foo/bar/spam/eggs").unwrap();

        assert_eq!(
            bar_id,
            table.get_directory_from_path("foo/bar/spam/../").unwrap()
        );
        assert_eq!(
            bar_id,
            table.get_directory_from_path("foo/bar/spam/../.").unwrap()
        );
        assert_eq!(
            bar_id,
            table
                .get_directory_from_path("foo/bar/.././bar/spam/../")
                .unwrap()
        );
        assert_eq!(
            bar_id,
            table
                .get_directory_from_path("./foo/.././foo/bar/spam/../")
                .unwrap()
        );
    }

    #[test]
    fn path() {
        let mut table = FsTable::new();

        let foo_id = table.create_dir("foo").unwrap();
        let bar_id = table.create_dir("foo/bar").unwrap();
        let spam_id = table.create_file("foo/bar/spam.txt").unwrap();
        let eggs_id = table.create_file("foo/eggs.txt").unwrap();
        let spamandeggs_id = table.create_file("spamandeggs.txt").unwrap();

        assert_eq!("foo", table.get_directory_path(foo_id).unwrap());
        assert_eq!("foo/bar", table.get_directory_path(bar_id).unwrap());
        assert_eq!("foo/bar/spam.txt", table.get_file_path(spam_id).unwrap());
        assert_eq!("foo/eggs.txt", table.get_file_path(eggs_id).unwrap());
        assert_eq!(
            "spamandeggs.txt",
            table.get_file_path(spamandeggs_id).unwrap()
        );
    }
}

#[cfg(test)]
mod util_tests {
    use crate::util;

    #[test]
    fn name_split() {
        let path_foobar = "foo/bar.txt";
        let path_spameggs = "spam/eggs";
        let path_root = "root.txt";

        assert_eq!(
            ("foo".to_string(), "bar.txt".to_string()),
            util::split_name_path(&path_foobar)
        );
        assert_eq!(
            ("spam".to_string(), "eggs".to_string()),
            util::split_name_path(&path_spameggs)
        );
        assert_eq!(
            ("".to_string(), "root.txt".to_string()),
            util::split_name_path(&path_root)
        );
    }
}

#[cfg(test)]
mod ramfs_tests {
    use crate::{fshandler::FsHandler, ramfs::RamFsHandler};

    #[test]
    fn create() {
        let mut fs = RamFsHandler::new();
        let spam_id = fs.create_directory("spam").unwrap();
        let eggs_id = fs.create_directory("spam/eggs").unwrap();

        let foo_id = fs.create_file("spam/eggs/foo.txt").unwrap();
        let bar_id = fs.create_file("bar.txt").unwrap();

        assert_eq!(spam_id, fs.get_dir("spam").unwrap());
        assert_eq!(eggs_id, fs.get_dir("spam/eggs").unwrap());
        assert_eq!(foo_id, fs.get_file("spam/eggs/foo.txt").unwrap());
        assert_eq!(bar_id, fs.get_file("bar.txt").unwrap());
    }

    #[test]
    fn copy_file() {
        let mut fs = RamFsHandler::new();

        fs.create_directory("spam/").unwrap();
        fs.create_directory("foo/").unwrap();

        let test_string = String::from("Hello, world!");

        let eggs_id = fs.create_file("spam/eggs.txt").unwrap();
        fs.write(eggs_id, 0, test_string.as_bytes()).unwrap();
        let bar_id = fs.copy_file("spam/eggs.txt", "foo/bar.txt").unwrap();

        assert_eq!(
            test_string,
            String::from_utf8_lossy(&fs.read(eggs_id).unwrap())
        );
        assert_eq!(fs.read(eggs_id).unwrap(), fs.read(bar_id).unwrap());
    }

    #[test]
    fn move_file() {
        let mut fs = RamFsHandler::new();

        fs.create_directory("spam/").unwrap();
        fs.create_directory("foo/").unwrap();

        let test_string = String::from("Hello, world!");

        let eggs_id = fs.create_file("spam/eggs.txt").unwrap();
        fs.write(eggs_id, 0, test_string.as_bytes()).unwrap();
        fs.move_file("spam/eggs.txt", "foo/bar.txt").unwrap();

        assert_eq!(
            test_string,
            String::from_utf8_lossy(&fs.read(eggs_id).unwrap())
        );
        assert_eq!(None, fs.get_file("spam/eggs.txt").ok());
    }

    #[test]
    fn copy_dir() {
        let mut fs = RamFsHandler::new();

        fs.create_directory("spam/").unwrap();
        fs.create_directory("spam/eggs").unwrap();
        fs.create_file("spam/eggs/spameggs.txt").unwrap();

        fs.copy_directory("spam/", "foo/").unwrap();

        fs.get_file("foo/eggs/spameggs.txt").unwrap();
        fs.get_file("spam/eggs/spameggs.txt").unwrap();
    }

    #[test]
    fn move_dir() {
        let mut fs = RamFsHandler::new();

        fs.create_directory("spam/").unwrap();
        fs.create_directory("spam/eggs").unwrap();
        fs.create_file("spam/eggs/spameggs.txt").unwrap();

        fs.move_directory("spam", "foo").unwrap();

        fs.get_file("foo/eggs/spameggs.txt").unwrap();
        assert_eq!(None, fs.get_file("spam/eggs/spameggs.txt").ok());
    }
}
