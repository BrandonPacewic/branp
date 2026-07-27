use std::collections::{HashMap, VecDeque};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use memchr::memchr;

use crate::context::GlobalContext;
use crate::errors::CliResult;
use crate::utils::terminal::{supports_dynamic_lines, DynamicRenderLoop};

mod languages;

const LANGUAGE_COLUMN_WIDTH: usize = 36;
const DEFAULT_NUMBER_COLUMN_WIDTH: usize = 12;
const NUMBER_COLUMNS: usize = 4;
const DEFAULT_NUMBER_COLUMN_DIGITS: usize = 8;
const READ_BUFFER_SIZE: usize = 256 * 1024;
const INITIAL_DISCOVERY_ITERATIONS: usize = 128;
const WORK_QUEUE_BATCH_SIZE: usize = 64;
const LIVE_RENDER_INTERVAL: Duration = Duration::from_millis(50);

pub struct ClocOptions<'a> {
    pub paths: Vec<&'a str>,
    pub live: bool,
    pub commas: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BenchCountReport {
    pub text_files: u64,
    pub ignored_files: u64,
    pub blank: u64,
    pub comment: u64,
    pub code: u64,
}

#[derive(Clone, Copy)]
struct RenderOptions {
    commas: bool,
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
    syntax: Option<&'static languages::CommentSyntax>,
}

#[derive(Clone, Debug, Default)]
struct DiscoveryStats {
    text_files: u64,
    ignored_files: u64,
}

#[derive(Clone, Debug, Default)]
struct CountReport {
    by_language: HashMap<&'static str, Count>,
    stats: DiscoveryStats,
    elapsed: Duration,
}

impl Count {
    fn add(&mut self, other: Self) {
        self.files += other.files;
        self.blank += other.blank;
        self.comment += other.comment;
        self.code += other.code;
    }

    fn lines(self) -> u64 {
        self.blank + self.comment + self.code
    }
}

pub fn cloc(gctx: &mut GlobalContext, options: &ClocOptions<'_>) -> CliResult {
    let paths = options.paths.iter().map(PathBuf::from).collect::<Vec<_>>();

    if gctx.shell().is_quiet() {
        let _ = count_paths(&paths);
    } else if options.live && supports_dynamic_lines() {
        let _ = count_paths_live(
            &paths,
            RenderOptions {
                commas: options.commas,
            },
        )?;
    } else {
        let report = count_paths(&paths);
        print_report(
            &report,
            RenderOptions {
                commas: options.commas,
            },
        );
    }

    Ok(())
}

pub fn count_paths_for_bench(paths: &[PathBuf]) -> BenchCountReport {
    let report = count_paths(paths);
    let total = total_count(&report.by_language);
    BenchCountReport {
        text_files: report.stats.text_files,
        ignored_files: report.stats.ignored_files,
        blank: total.blank,
        comment: total.comment,
        code: total.code,
    }
}

fn count_paths(paths: &[PathBuf]) -> CountReport {
    let start = Instant::now();
    let (by_language, stats, count_ignored_files) = count_paths_parallel(paths);
    CountReport {
        by_language,
        stats: DiscoveryStats {
            text_files: stats.text_files.saturating_sub(count_ignored_files),
            ignored_files: stats.ignored_files + count_ignored_files,
        },
        elapsed: start.elapsed(),
    }
}

fn count_paths_live(paths: &[PathBuf], render_options: RenderOptions) -> io::Result<CountReport> {
    let start = Instant::now();
    let initial = discover_initial_files(paths);
    let worker_count = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .max(1);
    let work_queue = Arc::new(WorkQueue::new(initial.files.clone()));
    let (sender, receiver) = mpsc::channel();
    let mut state = LiveState::new(&initial);
    let mut frame = DynamicRenderLoop::start(&state.render(start.elapsed(), render_options))?;
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
                        count_file(&file.path, file.syntax)
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
                        frame.advance(&state.render(start.elapsed(), render_options))?;
                        last_render = Instant::now();
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    frame.advance(&state.render(start.elapsed(), render_options))?;
                    last_render = Instant::now();
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        let report = state.into_report(start.elapsed());
        frame.replace(&render_report(&report, render_options))?;
        Ok(report)
    })
}

#[derive(Debug)]
struct InitialDiscovery {
    files: Vec<CountedFile>,
    queue: VecDeque<PathBuf>,
    language_files: HashMap<&'static str, u64>,
    stats: DiscoveryStats,
}

fn discover_initial_files(paths: &[PathBuf]) -> InitialDiscovery {
    let mut files = Vec::new();
    let mut language_files = HashMap::new();
    let mut stats = DiscoveryStats::default();
    let mut queue = paths.iter().cloned().collect::<VecDeque<_>>();
    let mut iterations = 0;

    while iterations < INITIAL_DISCOVERY_ITERATIONS {
        let Some(path) = queue.pop_front() else {
            break;
        };
        iterations += 1;

        discover_path(
            path,
            &mut queue,
            |file| {
                stats.text_files += 1;
                *language_files.entry(file.language).or_insert(0) += 1;
                files.push(file);
            },
            || {
                stats.ignored_files += 1;
            },
        );
    }

    InitialDiscovery {
        files,
        queue,
        language_files,
        stats,
    }
}

fn discover_remaining_files(
    mut discovery_queue: VecDeque<PathBuf>,
    work_queue: &WorkQueue,
    sender: &mpsc::Sender<LiveEvent>,
) {
    while let Some(path) = discovery_queue.pop_front() {
        discover_path(
            path,
            &mut discovery_queue,
            |file| {
                let language = file.language;
                work_queue.push(file);
                let _ = sender.send(LiveEvent::FileDiscovered { language });
            },
            || {
                let _ = sender.send(LiveEvent::FileIgnored);
            },
        );
    }
}

fn discover_path(
    path: PathBuf,
    queue: &mut VecDeque<PathBuf>,
    mut on_file: impl FnMut(CountedFile),
    mut on_ignored_file: impl FnMut(),
) {
    let Ok(metadata) = fs::symlink_metadata(&path) else {
        return;
    };

    if metadata.is_file() {
        if should_skip_file(&path) {
            on_ignored_file();
            return;
        }
        if let Some(language) = detect_language(&path) {
            on_file(counted_file(path, language));
        } else {
            on_ignored_file();
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

    fn pop_batch(&self, files: &mut Vec<CountedFile>) -> bool {
        files.clear();

        let mut state = self.state.lock().unwrap();
        loop {
            if let Some(file) = state.files.pop_front() {
                files.push(file);
                while files.len() < WORK_QUEUE_BATCH_SIZE {
                    let Some(file) = state.files.pop_front() else {
                        break;
                    };
                    files.push(file);
                }
                return true;
            }
            if state.done {
                return false;
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
    FileIgnored,
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
    ignored_files: u64,
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
            ignored_files: initial.stats.ignored_files,
            discovery_done: initial.queue.is_empty(),
            workers_done: 0,
        }
    }

    fn apply(&mut self, event: LiveEvent) {
        match event {
            LiveEvent::FileDiscovered { language } => {
                self.ensure_language(language);
            }
            LiveEvent::FileIgnored => {
                self.ignored_files += 1;
            }
            LiveEvent::FileCounted { language, count } => {
                self.ensure_language(language);
                if let Some(count) = count {
                    self.by_language.entry(language).or_default().add(count);
                } else {
                    self.ignored_files += 1;
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

    fn render(&self, elapsed: Duration, options: RenderOptions) -> Vec<String> {
        render_report_with_rows(&self.to_report(elapsed), self.render_rows(false), options)
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

    fn to_report(&self, elapsed: Duration) -> CountReport {
        let text_files = total_count(&self.by_language).files;
        CountReport {
            by_language: self.by_language.clone(),
            stats: DiscoveryStats {
                text_files,
                ignored_files: self.ignored_files,
            },
            elapsed,
        }
    }

    fn into_report(self, elapsed: Duration) -> CountReport {
        self.to_report(elapsed)
    }

    fn render_rows(&self, final_render: bool) -> Vec<(&'static str, Count)> {
        self.render_languages(final_render)
            .into_iter()
            .map(|language| {
                (
                    language,
                    self.by_language.get(language).copied().unwrap_or_default(),
                )
            })
            .collect()
    }
}

fn detect_language(path: &Path) -> Option<&'static str> {
    let filename = path.file_name()?.to_str()?;

    if let Some(language) = languages::language_for_filename(filename) {
        return Some(canonical_language(language));
    }

    let mut dot_indices = [0; 3];
    let mut dot_count = 0;
    for (index, _) in filename.match_indices('.') {
        if dot_count < dot_indices.len() {
            dot_indices[dot_count] = index;
            dot_count += 1;
        } else {
            dot_indices.rotate_left(1);
            dot_indices[dot_indices.len() - 1] = index;
        }
    }

    for index in &dot_indices[..dot_count] {
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

fn count_paths_parallel(paths: &[PathBuf]) -> (HashMap<&'static str, Count>, DiscoveryStats, u64) {
    let worker_count = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .max(1);
    let work_queue = Arc::new(WorkQueue::new(Vec::new()));

    thread::scope(|scope| {
        let mut workers = Vec::with_capacity(worker_count);

        for _ in 0..worker_count {
            let work_queue = Arc::clone(&work_queue);
            workers.push(scope.spawn(move || {
                let mut by_language = HashMap::new();
                let mut ignored_files = 0;
                let mut files = Vec::with_capacity(WORK_QUEUE_BATCH_SIZE);
                while work_queue.pop_batch(&mut files) {
                    for file in files.drain(..) {
                        if let Ok(Some(file_count)) = count_file(&file.path, file.syntax) {
                            let count = by_language
                                .entry(file.language)
                                .or_insert_with(Count::default);
                            count.files += 1;
                            count.add(file_count);
                        } else {
                            ignored_files += 1;
                        }
                    }
                }
                (by_language, ignored_files)
            }));
        }

        let stats = collect_files_into_queue(paths, &work_queue);
        work_queue.finish();

        let mut total = HashMap::new();
        let mut ignored_files = 0;
        for worker in workers {
            if let Ok((by_language, worker_ignored_files)) = worker.join() {
                ignored_files += worker_ignored_files;
                for (language, count) in by_language {
                    total
                        .entry(language)
                        .or_insert_with(Count::default)
                        .add(count);
                }
            }
        }
        (total, stats, ignored_files)
    })
}

fn collect_files_into_queue(paths: &[PathBuf], work_queue: &WorkQueue) -> DiscoveryStats {
    let mut stats = DiscoveryStats::default();
    let mut stack = paths.iter().cloned().collect::<VecDeque<_>>();

    while let Some(path) = stack.pop_back() {
        discover_path(
            path,
            &mut stack,
            |file| {
                stats.text_files += 1;
                work_queue.push(file);
            },
            || {
                stats.ignored_files += 1;
            },
        );
    }

    stats
}

fn counted_file(path: PathBuf, language: &'static str) -> CountedFile {
    CountedFile {
        path,
        language,
        syntax: languages::comment_syntax_for_language(language),
    }
}

fn count_file(
    path: &Path,
    syntax: Option<&'static languages::CommentSyntax>,
) -> io::Result<Option<Count>> {
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

    Ok(Some(count_bytes(&bytes, syntax)))
}

fn print_report(report: &CountReport, options: RenderOptions) {
    for line in render_report(report, options) {
        println!("{line}");
    }
}

fn render_report(report: &CountReport, options: RenderOptions) -> Vec<String> {
    render_report_with_rows(report, sorted_rows(&report.by_language), options)
}

fn render_report_with_rows(
    report: &CountReport,
    rows: Vec<(&'static str, Count)>,
    options: RenderOptions,
) -> Vec<String> {
    let total = total_count(&report.by_language);
    let elapsed_secs = report.elapsed.as_secs_f64();
    let files_per_second = rate(report.stats.text_files, elapsed_secs);
    let lines_per_second = rate(total.lines(), elapsed_secs);
    let mut lines = Vec::new();

    lines.push(format!(
        "{:>8} text files.",
        format_u64(report.stats.text_files, options)
    ));
    lines.push(format!(
        "{:>8} files ignored.",
        format_u64(report.stats.ignored_files, options)
    ));
    lines.push(String::new());
    lines.push(format!(
        "{} s, {} files/s, {} lines/s",
        format_f64(elapsed_secs, 2, options),
        format_f64(files_per_second, 1, options),
        format_f64(lines_per_second, 1, options)
    ));
    lines.extend(render_table(rows, total, options));
    lines
}

fn sorted_rows(by_language: &HashMap<&'static str, Count>) -> Vec<(&'static str, Count)> {
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
    rows.into_iter()
        .map(|(language, count)| (*language, *count))
        .collect()
}

fn render_table(
    rows: impl IntoIterator<Item = (&'static str, Count)>,
    total: Count,
    options: RenderOptions,
) -> Vec<String> {
    let mut lines = Vec::new();
    let number_width = number_column_width(total);
    let table_width = LANGUAGE_COLUMN_WIDTH + NUMBER_COLUMNS * (number_width + 1);
    let separator = "-".repeat(table_width);

    lines.push(separator.clone());
    lines.push(format!(
        "{:<language_width$} {:>number_width$} {:>number_width$} {:>number_width$} {:>number_width$}",
        "Language",
        "files",
        "blank",
        "comment",
        "code",
        language_width = LANGUAGE_COLUMN_WIDTH,
        number_width = number_width,
    ));
    lines.push(separator.clone());

    for (language, count) in rows {
        lines.push(format!(
            "{:<language_width$} {:>number_width$} {:>number_width$} {:>number_width$} {:>number_width$}",
            language,
            format_u64(count.files, options),
            format_u64(count.blank, options),
            format_u64(count.comment, options),
            format_u64(count.code, options),
            language_width = LANGUAGE_COLUMN_WIDTH,
            number_width = number_width,
        ));
    }

    lines.push(separator.clone());
    lines.push(format!(
        "{:<language_width$} {:>number_width$} {:>number_width$} {:>number_width$} {:>number_width$}",
        "SUM:",
        format_u64(total.files, options),
        format_u64(total.blank, options),
        format_u64(total.comment, options),
        format_u64(total.code, options),
        language_width = LANGUAGE_COLUMN_WIDTH,
        number_width = number_width,
    ));
    lines.push(separator);
    lines
}

fn number_column_width(total: Count) -> usize {
    let max_count = [total.files, total.blank, total.comment, total.code]
        .into_iter()
        .max()
        .unwrap_or(0);
    DEFAULT_NUMBER_COLUMN_WIDTH
        + digit_count(max_count).saturating_sub(DEFAULT_NUMBER_COLUMN_DIGITS)
}

fn digit_count(value: u64) -> usize {
    if value == 0 {
        return 1;
    }

    let mut digits = 0;
    let mut value = value;
    while value > 0 {
        digits += 1;
        value /= 10;
    }
    digits
}

fn format_u64(value: u64, options: RenderOptions) -> String {
    if options.commas {
        format_integer_with_commas(value.to_string())
    } else {
        value.to_string()
    }
}

fn format_f64(value: f64, precision: usize, options: RenderOptions) -> String {
    let value = format!("{value:.precision$}");
    if !options.commas {
        return value;
    }

    let Some((integer, fraction)) = value.split_once('.') else {
        return format_integer_with_commas(value);
    };

    format!(
        "{}.{}",
        format_integer_with_commas(integer.to_string()),
        fraction
    )
}

fn format_integer_with_commas(value: String) -> String {
    let mut formatted = String::with_capacity(value.len() + value.len() / 3);
    let first_group_len = value.len() % 3;

    for (index, ch) in value.chars().enumerate() {
        if index > 0
            && (index == first_group_len
                || (index > first_group_len && (index - first_group_len).is_multiple_of(3)))
        {
            formatted.push(',');
        }
        formatted.push(ch);
    }

    formatted
}

fn total_count(by_language: &HashMap<&'static str, Count>) -> Count {
    let mut total = Count::default();
    for count in by_language.values() {
        total.add(*count);
    }
    total
}

fn rate(count: u64, elapsed_secs: f64) -> f64 {
    if elapsed_secs > 0.0 {
        count as f64 / elapsed_secs
    } else {
        0.0
    }
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
    let mut start = 0;
    while start < bytes.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }

    let mut end = bytes.len();
    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    &bytes[start..end]
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    memchr::memmem::find(haystack, needle).is_some()
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

        let report = count_paths(std::slice::from_ref(&root));
        let text = report.by_language.get("Text").copied().unwrap_or_default();

        assert_eq!(
            text,
            Count {
                files: 2,
                blank: 2,
                comment: 0,
                code: 3
            }
        );
        assert_eq!(report.stats.text_files, 2);
        assert_eq!(report.stats.ignored_files, 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn skips_vcs_directories() {
        let root = unique_temp_dir();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/config"), "hidden\n").unwrap();
        fs::write(root.join("visible.txt"), "visible\n").unwrap();

        let report = count_paths(std::slice::from_ref(&root));
        let text = report.by_language.get("Text").copied().unwrap_or_default();

        assert_eq!(
            text,
            Count {
                files: 1,
                blank: 0,
                comment: 0,
                code: 1
            }
        );
        assert_eq!(report.stats.text_files, 1);
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

    #[test]
    fn formats_integer_groups_with_commas() {
        assert_eq!(format_u64(82, RenderOptions { commas: true }), "82");
        assert_eq!(
            format_u64(19_008_687, RenderOptions { commas: true }),
            "19,008,687"
        );
        assert_eq!(
            format_u64(19_008_687, RenderOptions { commas: false }),
            "19008687"
        );
    }

    #[test]
    fn expands_table_width_starting_at_nine_digits() {
        let eight_digits = Count {
            files: 1,
            blank: 1,
            comment: 1,
            code: 99_999_999,
        };
        let nine_digits = Count {
            files: 1,
            blank: 1,
            comment: 1,
            code: 100_000_000,
        };

        let eight_digit_lines = render_table(
            [("Rust", eight_digits)],
            eight_digits,
            RenderOptions { commas: true },
        );
        let nine_digit_lines = render_table(
            [("Rust", nine_digits)],
            nine_digits,
            RenderOptions { commas: true },
        );

        assert_eq!(eight_digit_lines[0].len(), 88);
        assert_eq!(nine_digit_lines[0].len(), 92);
        assert!(nine_digit_lines
            .iter()
            .any(|line| line.contains("100,000,000")));
    }

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bp-cloc-test-{nanos}"))
    }
}
