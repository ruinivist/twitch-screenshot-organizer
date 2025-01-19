use clap::Parser;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::fs::read_dir;
use std::io;
use std::path::Path;
use std::thread;

#[derive(Parser, Debug)]
struct Args {
    /// Path to process for twitch screenshots in
    path: String,

    /// Watch mode. If enabled, program will keep running and watch for new screenshots to move
    #[clap(short, long)]
    watch: bool,
}

fn main() {
    let args = Args::parse();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Watching {} for new screenshots to process...", args.path);
    log::debug!("Args were: {:?}", args);

    let handle = move_all(&args.path);

    if args.watch {
        if let Err(error) = run_as_daemon(args.path) {
            log::error!("Error: {error:?}");
        }
    }

    handle.join().expect("Failed to join on move all op");
}

/// for all files in the directory ( non recursive ) move to appropriate folder if it's a screenshot
/// in a separate thread
fn move_all<P: AsRef<Path>>(path: P) -> thread::JoinHandle<()> {
    let path = path.as_ref().to_path_buf();
    return thread::spawn(move || {
        read_dir(path)
            .expect("Failed to read directory")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
            .for_each(|entry| {
                let path = entry.path();
                if is_screenshot(&path) {
                    log::info!("Moving screenshot: {}", path.display());
                    if let Err(error) = move_file(&path, false) {
                        log::error!("Error: {error:?}");
                    }
                }
            });
    });
}

/// Watch for new screenshots in the directory and move them to appropriate folder
fn run_as_daemon<P: AsRef<Path>>(path: P) -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    // pick whatever is the best implfementation for system
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;

    for res in rx {
        match res {
            Ok(Event {
                kind: EventKind::Create(_),
                paths,
                ..
            }) => {
                for path in paths {
                    log::debug!("Processing: {}", path.display());
                    if is_screenshot(&path) {
                        log::info!("Moving screenshot: {}", path.display());
                        if let Err(error) = move_file(&path, true) {
                            log::error!("Error: {error:?}");
                        }
                    }
                }
            }
            Ok(_) => {} // Ignore other kind of events
            Err(error) => log::error!("Error: {error:?}"),
        }
    }

    Ok(())
}

/// Simple heuristic to determine if a file is a twitch screenshot
fn is_screenshot(path: &Path) -> bool {
    let filename = path
        .file_name()
        .unwrap()
        .to_str()
        .expect("Invalid filename");

    // its a png
    if !filename.ends_with(".png") {
        return false;
    }

    // remove png
    let filename = filename.strip_suffix(".png").unwrap();

    // that has three _
    let parts: Vec<&str> = filename.split('_').collect();
    if parts.len() < 5 {
        // not equal to 5 because the channel name can have _ in it
        return false;
    }

    let splits = parts.len();
    // -3 to end is time
    let time = parts[splits - 3..].join("_");
    // -4 is date
    let date = parts[splits - 4];
    // channel name would be 0 to -5 joined _

    // its of format Sat-Jan-18-2025
    if date.len() != 15 || date.split('-').count() != 4 {
        return false;
    }

    // remove the possible duplicate number suffix like (1) or (2)
    let time = time.split('(').next().unwrap();
    // this should look like 1_06_05-PM or 12_06_05-AM
    if (time.len() == 10 || time.len() == 11) && time.split('_').count() != 3 {
        return false;
    }

    // this is now very likely a screenshot
    return true;
}

/// move the file to [SAVE_TO]/[channel_name]/[filename]
fn move_file(file_path: &Path, daemon_mode: bool) -> io::Result<()> {
    let parent_dir = file_path.parent().expect("File has no parent directory");
    let channel_name = channel_name(file_path.file_name().unwrap().to_str().unwrap());

    const SAVE_TO: &str = "twitch-screenshots";

    let target_dir = parent_dir.join(SAVE_TO).join(channel_name);
    fs::create_dir_all(&target_dir)?; // Ensure the target directory exists
    let file_name = file_path.file_name().unwrap();
    let target_file_path = target_dir.join(file_name);

    // Move the file after 2s to ensure it's fully written when moving
    let file_path_clone = file_path.to_path_buf();

    if daemon_mode {
        thread::spawn(move || {
            thread::sleep(std::time::Duration::from_secs(2));
            if let Err(e) = fs::rename(&file_path_clone, &target_file_path) {
                log::error!("Failed to move file: {}", e);
            } else {
                log::info!("File moved to: {}", target_file_path.to_string_lossy());
            }
        });
    } else {
        fs::rename(&file_path, &target_file_path)?;
        log::info!("File moved to: {}", target_file_path.to_string_lossy());
    }

    Ok(())
}

/// channel name from filename
fn channel_name(filename: &str) -> String {
    let parts = filename.split('_').collect::<Vec<&str>>();
    let channel_name = parts[0..parts.len() - 4].join("_");
    channel_name
}
