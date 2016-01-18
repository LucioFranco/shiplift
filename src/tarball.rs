
use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, MAIN_SEPARATOR};
use std::io;
use tar::Archive;

// todo: factor this into its own crate
pub fn dir(path: &str) -> io::Result<File> {
    let file = OpenOptions::new().read(true).write(true).create(true).open("build.tgz").unwrap();
    let zipper = GzEncoder::new(file, Compression::Best);
    let archive = Archive::new(zipper);
    fn bundle(dir: &Path, cb: &Fn(&Path), bundle_dir: bool) -> io::Result<()> {
        if try!(fs::metadata(dir)).is_dir() {
            if bundle_dir {
                cb(&dir);
            }
            for entry in try!(fs::read_dir(dir)) {
                let entry = try!(entry);
                if try!(fs::metadata(entry.path())).is_dir() {
                    try!(bundle(&entry.path(), cb, true));
                } else {
                    cb(&entry.path().as_path());
                }
            }
        }
        Ok(())
    }

    {
        let base_path = Path::new(path).canonicalize().unwrap();
        let mut base_path_str = base_path.to_str().unwrap().to_owned();
        if base_path_str.chars().last().unwrap() != MAIN_SEPARATOR {
            base_path_str.push(MAIN_SEPARATOR)
        }

        let append = |path: &Path| {
            let canonical = path.canonicalize().unwrap();
            let relativized = canonical.to_str().unwrap().trim_left_matches(&base_path_str[..]);
            if path.is_dir() {
                archive.append_dir(Path::new(relativized), &canonical).unwrap();
            } else {
                archive.append_file(Path::new(relativized), &mut File::open(&canonical).unwrap()).unwrap();
            }
        };
        try!(bundle(Path::new(path), &append, false));
        try!(archive.finish());
    }
    File::open("build.tgz")
}
