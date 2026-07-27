use std::collections::{HashMap, VecDeque};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use memchr::memchr;

use crate::context::GlobalContext;
use crate::errors::CliResult;
use crate::utils::terminal::{supports_dynamic_lines, DynamicRenderLoop};

mod languages;

const READ_BUFFER_SIZE: usize = 256 * 1024;
const INITIAL_DISCOVERY_ITERATIONS: usize = 128;
const LIVE_RENDER_INTERVAL: Duration = Duration::from_millis(50);

pub struct ClocOptions<'a> {
    pub paths: Vec<&'a str>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Count {
    files: u64,
    blank: u64,
    comment: u64,
    code: u64,
}

#[derive(Clone, Debug)]
struct CountedFile {
    path: PathBuf,
    language: &'static str,
}

impl Count {
    fn add(&mut self, other: Self) {
        self.files += other.files;
        self.blank += other.blank;
        self.comment += other.comment;
        self.code += other.code;
    }
}

pub fn cloc(gctx: &mut GlobalContext, options: &ClocOptions<'_>) -> CliResult {
    let paths = options.paths.iter().map(PathBuf::from).collect::<Vec<_>>();

    if gctx.shell().is_quiet() {
        let _ = count_paths(&paths);
    } else if supports_dynamic_lines() {
        count_paths_live(&paths)?;
    } else {
        let by_language = count_paths(&paths);
        print_report(&by_language);
    }

    Ok(())
}

fn count_paths(paths: &[PathBuf]) -> HashMap<&'static str, Count> {
    let files = collect_files(paths);
    count_files_parallel(files)
}

fn count_paths_live(paths: &[PathBuf]) -> io::Result<HashMap<&'static str, Count>> {
    let initial = discover_initial_files(paths);
    let worker_count = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .max(1);
    let work_queue = Arc::new(WorkQueue::new(initial.files.clone()));
    let (sender, receiver) = mpsc::channel();
    let mut state = LiveState::new(&initial);
    let mut frame = DynamicRenderLoop::start(&state.render(Some('-')))?;
    let mut last_render = Instant::now();

    thread::scope(|scope| {
        {
            let work_queue = Arc::clone(&work_queue);
            let sender = sender.clone();
            let discovery_queue = initial.queue.clone();
            scope.spawn(move || {
                discover_remaining_files(discovery_queue, &work_queue, &sender);
                work_queue.finish();
                let _ = sender.send(LiveEvent::DiscoveryDone);
            });
        }

        for _ in 0..worker_count {
            let work_queue = Arc::clone(&work_queue);
            let sender = sender.clone();
            scope.spawn(move || {
                while let Some(file) = work_queue.pop() {
                    let count =
                        count_file(&file.path, file.language)
                            .ok()
                            .flatten()
                            .map(|mut count| {
                                count.files = 1;
                                count
                            });
                    let _ = sender.send(LiveEvent::FileCounted {
                        language: file.language,
                        count,
                    });
                }
                let _ = sender.send(LiveEvent::WorkerDone);
            });
        }

        drop(sender);

        while !state.is_done(worker_count) {
            match receiver.recv_timeout(frame.tick_interval()) {
                Ok(event) => {
                    state.apply(event);
                    while let Ok(event) = receiver.try_recv() {
                        state.apply(event);
                    }
                    if state.is_done(worker_count) || last_render.elapsed() >= LIVE_RENDER_INTERVAL
                    {
                        frame.advance(&state.render(Some(frame.spinner())))?;
                        last_render = Instant::now();
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    frame.advance(&state.render(Some(frame.spinner())))?;
                    last_render = Instant::now();
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        frame.replace(&state.render(None))?;
        Ok(state.by_language)
    })
}

#[derive(Debug)]
struct InitialDiscovery {
    files: Vec<CountedFile>,
    queue: VecDeque<PathBuf>,
    language_files: HashMap<&'static str, u64>,
}

fn discover_initial_files(paths: &[PathBuf]) -> InitialDiscovery {
    let mut files = Vec::new();
    let mut language_files = HashMap::new();
    let mut queue = paths.iter().cloned().collect::<VecDeque<_>>();
    let mut iterations = 0;

    while iterations < INITIAL_DISCOVERY_ITERATIONS {
        let Some(path) = queue.pop_front() else {
            break;
        };
        iterations += 1;

        discover_path(path, &mut queue, |file| {
            *language_files.entry(file.language).or_insert(0) += 1;
            files.push(file);
        });
    }

    InitialDiscovery {
        files,
        queue,
        language_files,
    }
}

fn discover_remaining_files(
    mut discovery_queue: VecDeque<PathBuf>,
    work_queue: &WorkQueue,
    sender: &mpsc::Sender<LiveEvent>,
) {
    while let Some(path) = discovery_queue.pop_front() {
        discover_path(path, &mut discovery_queue, |file| {
            let language = file.language;
            work_queue.push(file);
            let _ = sender.send(LiveEvent::FileDiscovered { language });
        });
    }
}

fn discover_path(
    path: PathBuf,
    queue: &mut VecDeque<PathBuf>,
    mut on_file: impl FnMut(CountedFile),
) {
    let Ok(metadata) = fs::symlink_metadata(&path) else {
        return;
    };

    if metadata.is_file() && !should_skip_file(&path) {
        if let Some(language) = detect_language(&path) {
            on_file(CountedFile { path, language });
        }
        return;
    }

    if !metadata.is_dir() || should_skip_dir(&path) {
        return;
    }

    let Ok(entries) = fs::read_dir(&path) else {
        return;
    };

    for entry in entries.filter_map(Result::ok) {
        queue.push_back(entry.path());
    }
}

fn collect_files(paths: &[PathBuf]) -> Vec<CountedFile> {
    let mut files = Vec::new();
    let mut stack = paths.to_vec();

    while let Some(path) = stack.pop() {
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };

        if metadata.is_file() && !should_skip_file(&path) {
            if let Some(language) = detect_language(&path) {
                files.push(CountedFile { path, language });
            }
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

struct WorkQueue {
    state: Mutex<WorkQueueState>,
    available: Condvar,
}

struct WorkQueueState {
    files: VecDeque<CountedFile>,
    done: bool,
}

impl WorkQueue {
    fn new(files: Vec<CountedFile>) -> Self {
        Self {
            state: Mutex::new(WorkQueueState {
                files: files.into(),
                done: false,
            }),
            available: Condvar::new(),
        }
    }

    fn push(&self, file: CountedFile) {
        let mut state = self.state.lock().unwrap();
        state.files.push_back(file);
        self.available.notify_one();
    }

    fn pop(&self) -> Option<CountedFile> {
        let mut state = self.state.lock().unwrap();
        loop {
            if let Some(file) = state.files.pop_front() {
                return Some(file);
            }
            if state.done {
                return None;
            }
            state = self.available.wait(state).unwrap();
        }
    }

    fn finish(&self) {
        let mut state = self.state.lock().unwrap();
        state.done = true;
        self.available.notify_all();
    }
}

enum LiveEvent {
    FileDiscovered {
        language: &'static str,
    },
    FileCounted {
        language: &'static str,
        count: Option<Count>,
    },
    DiscoveryDone,
    WorkerDone,
}

struct LiveState {
    by_language: HashMap<&'static str, Count>,
    ordered_languages: Vec<&'static str>,
    discovered_files: u64,
    counted_files: u64,
    discovery_done: bool,
    workers_done: usize,
}

impl LiveState {
    fn new(initial: &InitialDiscovery) -> Self {
        let mut ordered_languages = initial.language_files.iter().collect::<Vec<_>>();
        ordered_languages.sort_by(
            |(left_language, left_files), (right_language, right_files)| {
                right_files
                    .cmp(left_files)
                    .then_with(|| left_language.cmp(right_language))
            },
        );
        let ordered_languages = ordered_languages
            .into_iter()
            .map(|(language, _)| *language)
            .collect();

        Self {
            by_language: HashMap::new(),
            ordered_languages,
            discovered_files: initial.files.len() as u64,
            counted_files: 0,
            discovery_done: initial.queue.is_empty(),
            workers_done: 0,
        }
    }

    fn apply(&mut self, event: LiveEvent) {
        match event {
            LiveEvent::FileDiscovered { language } => {
                self.discovered_files += 1;
                self.ensure_language(language);
            }
            LiveEvent::FileCounted { language, count } => {
                self.counted_files += 1;
                self.ensure_language(language);
                if let Some(count) = count {
                    self.by_language.entry(language).or_default().add(count);
                }
            }
            LiveEvent::DiscoveryDone => {
                self.discovery_done = true;
            }
            LiveEvent::WorkerDone => {
                self.workers_done += 1;
            }
        }
    }

    fn ensure_language(&mut self, language: &'static str) {
        if !self.ordered_languages.contains(&language) {
            self.ordered_languages.push(language);
        }
    }

    fn is_done(&self, worker_count: usize) -> bool {
        self.discovery_done && self.workers_done >= worker_count
    }

    fn render(&self, spinner: Option<char>) -> Vec<String> {
        let mut lines = Vec::new();
        let status = match spinner {
            Some(spinner) if self.discovery_done => {
                format!(
                    "{spinner} counting files: {}/{}",
                    self.counted_files, self.discovered_files
                )
            }
            Some(spinner) => format!(
                "{spinner} scanning and counting files: counted {}, found {}",
                self.counted_files, self.discovered_files
            ),
            None => format!("counted {} files", self.counted_files),
        };
        let render_languages = self.render_languages(spinner.is_none());

        lines.push(status);
        lines.push(format!(
            "{:<28} {:>12} {:>12} {:>12} {:>12}",
            "Language", "files", "blank", "comment", "code"
        ));
        lines.push(format!(
            "{:<28} {:>12} {:>12} {:>12} {:>12}",
            "--------", "-----", "-----", "-------", "----"
        ));

        let mut total = Count::default();
        for language in render_languages {
            let count = self.by_language.get(language).copied().unwrap_or_default();
            total.add(count);
            lines.push(format!(
                "{:<28} {:>12} {:>12} {:>12} {:>12}",
                language, count.files, count.blank, count.comment, count.code
            ));
        }

        lines.push(format!(
            "{:<28} {:>12} {:>12} {:>12} {:>12}",
            "SUM:", total.files, total.blank, total.comment, total.code
        ));
        lines
    }

    fn render_languages(&self, final_render: bool) -> Vec<&'static str> {
        let mut languages = self.ordered_languages.clone();
        if final_render {
            languages.retain(|language| {
                self.by_language
                    .get(language)
                    .map(|count| count.files > 0)
                    .unwrap_or(false)
            });
        }
        languages.sort_by(|left_language, right_language| {
            let left_count = self
                .by_language
                .get(left_language)
                .copied()
                .unwrap_or_default();
            let right_count = self
                .by_language
                .get(right_language)
                .copied()
                .unwrap_or_default();
            right_count
                .code
                .cmp(&left_count.code)
                .then_with(|| right_count.files.cmp(&left_count.files))
                .then_with(|| {
                    self.language_index(left_language)
                        .cmp(&self.language_index(right_language))
                })
                .then_with(|| left_language.cmp(right_language))
        });
        languages
    }

    fn language_index(&self, language: &'static str) -> usize {
        self.ordered_languages
            .iter()
            .position(|candidate| *candidate == language)
            .unwrap_or(usize::MAX)
    }
}

fn detect_language(path: &Path) -> Option<&'static str> {
    let filename = path.file_name()?.to_str()?;

    if let Some(language) = languages::language_for_filename(filename) {
        return Some(canonical_language(language));
    }

    let dot_indices = filename
        .match_indices('.')
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    for index in dot_indices.iter().rev().take(3).rev() {
        let suffix = &filename[index + 1..];
        if let Some(language) = languages::language_for_extension(suffix) {
            return Some(canonical_language(language));
        }
    }

    let stem = filename.split_once('.').map(|(stem, _)| stem)?;
    languages::language_for_prefix(stem).map(canonical_language)
}

fn canonical_language(language: &'static str) -> &'static str {
    match language {
        "Ant/XML" => "Ant",
        "C#/Smalltalk" => "C#",
        "Clojure/Cangjie" => "Clojure",
        "D/dtrace" => "D",
        "F#/Forth" => "F#",
        "Fortran 77/Forth" => "Fortran 77",
        "IDL/Qt Project/Prolog/ProGuard" => "IDL",
        "Lisp/Julia" => "Lisp",
        "Lisp/OpenCL" => "Lisp",
        "MATLAB/Mathematica/Objective-C/MUMPS/Mercury" => "MATLAB",
        "Maven/XML" => "Maven",
        "Pascal/Pawn" => "Pascal",
        "Pascal/Puppet" => "Pascal",
        "Perl/Prolog" => "Perl",
        "PHP/Pascal/Fortran/Pawn" => "PHP",
        "Raku/Prolog" => "Raku",
        "Scheme/SaltStack" => "Scheme",
        "SKILL/.NET IL" => "SKILL",
        "TypeScript/Qt Linguist" => "TypeScript",
        "Verilog-SystemVerilog/Coq" => "Verilog-SystemVerilog",
        "Visual Basic/TeX/Apex Class" => "Visual Basic",
        "XML-Qt-GTK/Glade" => "Glade",
        _ => language,
    }
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

fn count_files_parallel(files: Vec<CountedFile>) -> HashMap<&'static str, Count> {
    if files.is_empty() {
        return HashMap::new();
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
                let mut by_language = HashMap::new();
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(file) = files.get(index) else {
                        break;
                    };

                    if let Ok(Some(file_count)) = count_file(&file.path, file.language) {
                        let count = by_language
                            .entry(file.language)
                            .or_insert_with(Count::default);
                        count.files += 1;
                        count.add(file_count);
                    }
                }
                by_language
            }));
        }

        let mut total = HashMap::new();
        for worker in workers {
            if let Ok(by_language) = worker.join() {
                for (language, count) in by_language {
                    total
                        .entry(language)
                        .or_insert_with(Count::default)
                        .add(count);
                }
            }
        }
        total
    })
}

fn count_file(path: &Path, language: &str) -> io::Result<Option<Count>> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    let mut buffer = [0; READ_BUFFER_SIZE];

    loop {
        let len = file.read(&mut buffer)?;
        if len == 0 {
            break;
        }

        let chunk = &buffer[..len];
        if memchr(0, chunk).is_some() {
            return Ok(None);
        }
        bytes.extend_from_slice(chunk);
    }

    Ok(Some(count_bytes(
        &bytes,
        languages::comment_syntax_for_language(language),
    )))
}

fn print_report(by_language: &HashMap<&'static str, Count>) {
    let mut rows = by_language.iter().collect::<Vec<_>>();
    rows.sort_by(
        |(left_language, left_count), (right_language, right_count)| {
            right_count
                .code
                .cmp(&left_count.code)
                .then_with(|| right_count.files.cmp(&left_count.files))
                .then_with(|| left_language.cmp(right_language))
        },
    );

    println!(
        "{:<28} {:>12} {:>12} {:>12} {:>12}",
        "Language", "files", "blank", "comment", "code"
    );
    println!(
        "{:<28} {:>12} {:>12} {:>12} {:>12}",
        "--------", "-----", "-----", "-------", "----"
    );

    let mut total = Count::default();
    for (language, count) in rows {
        total.add(*count);
        println!(
            "{:<28} {:>12} {:>12} {:>12} {:>12}",
            language, count.files, count.blank, count.comment, count.code
        );
    }

    println!(
        "{:<28} {:>12} {:>12} {:>12} {:>12}",
        "SUM:", total.files, total.blank, total.comment, total.code
    );
}

fn count_bytes(bytes: &[u8], syntax: Option<&languages::CommentSyntax>) -> Count {
    if bytes.is_empty() {
        return Count::default();
    }

    let mut count = Count::default();
    let mut block_comment: Option<&languages::BlockComment> = None;

    let mut start = 0;
    while start < bytes.len() {
        let newline = memchr(b'\n', &bytes[start..]).map(|offset| start + offset);
        let end = newline.unwrap_or(bytes.len());
        let line = &bytes[start..end];
        start = newline.map_or(bytes.len(), |index| index + 1);

        let trimmed = trim_ascii(line);
        if trimmed.is_empty() {
            count.blank += 1;
            continue;
        }

        let Some(syntax) = syntax else {
            count.code += 1;
            continue;
        };

        if let Some(block) = block_comment {
            count.comment += 1;
            if contains_bytes(trimmed, block.end.as_bytes()) {
                block_comment = None;
            }
            continue;
        }

        if syntax
            .line_markers
            .iter()
            .any(|marker| trimmed.starts_with(marker.as_bytes()))
        {
            count.comment += 1;
            continue;
        }

        if let Some(block) = syntax
            .block_markers
            .iter()
            .find(|block| trimmed.starts_with(block.start.as_bytes()))
        {
            count.comment += 1;
            if !contains_bytes(trimmed, block.end.as_bytes()) {
                block_comment = Some(block);
            }
            continue;
        }

        count.code += 1;
    }

    count
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map(|index| index + 1)
        .unwrap_or(start);
    &bytes[start..end]
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }

    haystack
        .windows(needle.len())
        .any(|window| window == needle)
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
        let text = count.get("Text").copied().unwrap_or_default();

        assert_eq!(
            text,
            Count {
                files: 2,
                blank: 2,
                comment: 0,
                code: 3
            }
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn skips_vcs_directories() {
        let root = unique_temp_dir();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/config"), "hidden\n").unwrap();
        fs::write(root.join("visible.txt"), "visible\n").unwrap();

        let count = count_paths(&[root.clone()]);
        let text = count.get("Text").copied().unwrap_or_default();

        assert_eq!(
            text,
            Count {
                files: 1,
                blank: 0,
                comment: 0,
                code: 1
            }
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn detects_cloc_languages_by_filename_and_long_suffix() {
        assert_eq!(detect_language(Path::new("Makefile")), Some("make"));
        assert_eq!(detect_language(Path::new("main.rs.in")), Some("Rust"));
        assert_eq!(detect_language(Path::new("trace.d")), Some("D"));
        assert_eq!(
            detect_language(Path::new("Dockerfile.test")),
            Some("Dockerfile")
        );
        assert_eq!(detect_language(Path::new("unknown.not-code")), None);
    }

    #[test]
    fn counts_blank_comment_and_code_lines() {
        let count = count_bytes(
            b"// header\n\nfn main() {\n    println!(\"hi\"); // inline\n}\n/* block\ncontinued\n*/\n",
            languages::comment_syntax_for_language("Rust"),
        );

        assert_eq!(
            count,
            Count {
                files: 0,
                blank: 1,
                comment: 4,
                code: 3
            }
        );
    }

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bp-cloc-test-{nanos}"))
    }
}
