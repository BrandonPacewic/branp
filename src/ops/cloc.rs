use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use memchr::{memchr, memchr_iter};

use crate::context::GlobalContext;
use crate::errors::CliResult;

const READ_BUFFER_SIZE: usize = 256 * 1024;

pub struct ClocOptions<'a> {
    pub paths: Vec<&'a str>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Count {
    files: u64,
    lines: u64,
}

impl Count {
    fn add(&mut self, other: Self) {
        self.files += other.files;
        self.lines += other.lines;
    }
}

pub fn cloc(gctx: &mut GlobalContext, options: &ClocOptions<'_>) -> CliResult {
    let paths = options.paths.iter().map(PathBuf::from).collect::<Vec<_>>();
    let count = count_paths(&paths);

    if !gctx.shell().is_quiet() {
        print_report(count);
    }

    Ok(())
}

fn count_paths(paths: &[PathBuf]) -> Count {
    let files = collect_files(paths);
    count_files_parallel(files)
}

fn collect_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = paths.to_vec();

    while let Some(path) = stack.pop() {
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };

        if metadata.is_file() && !should_skip_file(&path) {
            files.push(path);
            continue;
        }

        if !metadata.is_dir() || should_skip_dir(&path) {
            continue;
        }

        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };

        for entry in entries.filter_map(Result::ok) {
            stack.push(entry.path());
        }
    }

    files
}

fn should_skip_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git" | ".hg" | ".svn")
    )
}

fn should_skip_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some(
            "a" | "bmp"
                | "class"
                | "dll"
                | "dylib"
                | "exe"
                | "gif"
                | "gz"
                | "ico"
                | "jpeg"
                | "jpg"
                | "o"
                | "pdf"
                | "png"
                | "rmeta"
                | "rlib"
                | "so"
                | "tar"
                | "wasm"
                | "webp"
                | "zip"
        )
    )
}

fn count_files_parallel(files: Vec<PathBuf>) -> Count {
    if files.is_empty() {
        return Count::default();
    }

    let worker_count = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(files.len());
    let files = Arc::new(files);
    let next = Arc::new(AtomicUsize::new(0));

    thread::scope(|scope| {
        let mut workers = Vec::with_capacity(worker_count);

        for _ in 0..worker_count {
            let files = Arc::clone(&files);
            let next = Arc::clone(&next);
            workers.push(scope.spawn(move || {
                let mut count = Count::default();
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(path) = files.get(index) else {
                        break;
                    };

                    if let Ok(Some(lines)) = count_file_lines(path) {
                        count.files += 1;
                        count.lines += lines;
                    }
                }
                count
            }));
        }

        let mut total = Count::default();
        for worker in workers {
            if let Ok(count) = worker.join() {
                total.add(count);
            }
        }
        total
    })
}

fn count_file_lines(path: &Path) -> io::Result<Option<u64>> {
    let mut file = File::open(path)?;
    let mut buffer = [0; READ_BUFFER_SIZE];
    let mut lines = 0;
    let mut saw_bytes = false;
    let mut last_byte = b'\n';

    loop {
        let len = file.read(&mut buffer)?;
        if len == 0 {
            break;
        }

        let bytes = &buffer[..len];
        if memchr(0, bytes).is_some() {
            return Ok(None);
        }

        saw_bytes = true;
        last_byte = bytes[len - 1];
        lines += memchr_iter(b'\n', bytes).count() as u64;
    }

    if saw_bytes && last_byte != b'\n' {
        lines += 1;
    }

    Ok(Some(lines))
}

fn print_report(count: Count) {
    println!("{:>12} {:>12}", "files", "lines");
    println!("{:>12} {:>12}", "-----", "-----");
    println!("{:>12} {:>12}", count.files, count.lines);
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn counts_files_recursively_and_skips_binary_files() {
        let root = unique_temp_dir();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("one.txt"), "one\ntwo\nthree").unwrap();
        fs::write(root.join("src/two.txt"), "\n\n").unwrap();
        fs::write(root.join("binary.bin"), b"one\0two\n").unwrap();
        fs::write(root.join("artifact.rlib"), b"not\ncounted\n").unwrap();

        let count = count_paths(&[root.clone()]);

        assert_eq!(count, Count { files: 2, lines: 5 });
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn skips_vcs_directories() {
        let root = unique_temp_dir();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/config"), "hidden\n").unwrap();
        fs::write(root.join("visible.txt"), "visible\n").unwrap();

        let count = count_paths(&[root.clone()]);

        assert_eq!(count, Count { files: 1, lines: 1 });
        fs::remove_dir_all(root).unwrap();
    }

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bp-cloc-test-{nanos}"))
    }
}
