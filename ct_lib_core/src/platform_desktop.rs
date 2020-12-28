use std::ffi::OsStr;
use std::path::Path;

pub use easy_process;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debugging and performance

static mut TIMER_STARTING_INSTANT: Option<std::time::Instant> = None;
pub fn timer_initialize() {
    unsafe {
        TIMER_STARTING_INSTANT = Some(std::time::Instant::now());
    }
}
pub fn timer_current_time_seconds() -> f64 {
    unsafe {
        std::time::Instant::now()
            .duration_since(
                TIMER_STARTING_INSTANT.expect("Timer needs to be initialized before use"),
            )
            .as_secs_f64()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Logger

pub fn init_logging(logfile_path: &str, loglevel: log::Level) -> Result<(), String> {
    let logfile = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(logfile_path)
        .map_err(|error| format!("Could not create logfile at '{}' : {}", logfile_path, error))?;

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}::{}: {}",
                record.target(),
                record.level(),
                message
            ))
        })
        .level(loglevel.to_level_filter())
        .level_for("gfx_backend_dx11", log::LevelFilter::Warn)
        .level_for("gfx_backend_vulkan", log::LevelFilter::Warn)
        .level_for("wgpu_native", log::LevelFilter::Warn)
        .level_for("rusty_xinput", log::LevelFilter::Info)
        .level_for("gilrs::gamepad", log::LevelFilter::Info)
        .level_for("gilrs::ff::server", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(logfile)
        .apply()
        .map_err(|error| format!("Could initialize logger: {}", error))?;

    log::info!("Logger initialized");

    Ok(())
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Fileloading

pub fn read_file_whole(filepath: &str) -> Result<Vec<u8>, String> {
    std::fs::read(filepath)
        .map_err(|error| format!("Could not read file '{}' : {}", filepath, error))
}

pub struct Fileloader {
    content: Option<Vec<u8>>,
    finished: bool,
}

impl Fileloader {
    pub fn new(filepath: &str) -> Result<Fileloader, String> {
        let content = std::fs::read(filepath)
            .map_err(|error| format!("Could not fetch file '{}' : {}", filepath, error))?;
        Ok(Fileloader {
            content: Some(content),
            finished: false,
        })
    }

    pub fn is_done(&self) -> bool {
        true
    }

    pub fn poll(&mut self) -> Result<Option<Vec<u8>>, String> {
        assert!(!self.finished);

        self.finished = true;
        Ok(self.content.take())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Paths

pub trait PathHelper {
    fn to_string_borrowed(&self) -> Result<&str, String>;

    fn to_string_owned(&self) -> Result<String, String> {
        self.to_string_borrowed().map(|result| result.to_string())
    }
    fn to_string_owned_or_panic(&self) -> String {
        self.to_string_owned().unwrap()
    }
    fn to_string_borrowed_or_panic(&self) -> &str {
        self.to_string_borrowed().unwrap()
    }
}

impl PathHelper for Path {
    fn to_string_borrowed(&self) -> Result<&str, String> {
        if let Some(result) = self.to_str() {
            Ok(result)
        } else {
            Err(format!("Could not convert path to String {:?}", self))
        }
    }
}

impl PathHelper for OsStr {
    fn to_string_borrowed(&self) -> Result<&str, String> {
        if let Some(result) = self.to_str() {
            Ok(result)
        } else {
            Err(format!("Could not convert path to String {:?}", self))
        }
    }
}

pub fn path_exists(path: &str) -> bool {
    Path::new(path).exists()
}

pub fn path_canonicalize(path: &str) -> Result<String, String> {
    let canonicalized = std::path::Path::new(path)
        .canonicalize()
        .map_err(|error| format!("Cannot canonocalize path '{}' : {}", path, error))?;
    let canonicalized_string = canonicalized.to_string_borrowed()?;

    // Remove extendet length path syntax on windows
    Ok(canonicalized_string.replace("\\\\?\\", ""))
}

pub fn path_dir_empty(dir_path: &str) -> bool {
    walkdir::WalkDir::new(dir_path)
        .into_iter()
        .filter_map(|maybe_entry| maybe_entry.ok())
        .filter(|entry| entry.file_name().to_string_borrowed_or_panic() != dir_path)
        .count()
        == 0
}

/// NOTE: Result contains Unix-style file seperators only
pub fn path_join(first: &str, second: &str) -> String {
    Path::new(first)
        .join(second)
        .to_string_owned_or_panic()
        .replace("\\", "/")
}

/// NOTE: Result contains Unix-style file seperators only
pub fn path_with_extension(filepath: &str, new_extension: &str) -> String {
    Path::new(filepath)
        .with_extension(new_extension)
        .to_string_owned_or_panic()
        .replace("\\", "/")
}

/// NOTE: Result contains Unix-style file seperators only
pub fn path_without_extension(filepath: &str) -> String {
    Path::new(filepath)
        .with_extension("")
        .to_string_owned_or_panic()
        .replace("\\", "/")
}

/// NOTE: Result contains Unix-style file seperators only
pub fn path_without_filename(filepath: &str) -> String {
    Path::new(filepath)
        .with_file_name("")
        .to_string_owned_or_panic()
        .replace("\\", "/")
}

/// NOTE: Result contains Unix-style file seperators only
pub fn path_to_extension(filepath: &str) -> String {
    Path::new(filepath)
        .extension()
        .unwrap_or_else(|| panic!("Could not retrieve filename from path {}", filepath))
        .to_string_owned_or_panic()
        .replace("\\", "/")
}

/// NOTE: Result contains Unix-style file seperators only
pub fn path_to_filename(filepath: &str) -> String {
    Path::new(filepath)
        .file_name()
        .unwrap_or_else(|| panic!("Could not retrieve filename from path {}", filepath))
        .to_string_owned_or_panic()
        .replace("\\", "/")
}

/// NOTE: Result contains Unix-style file seperators only
pub fn path_to_filename_without_extension(filepath: &str) -> String {
    Path::new(filepath)
        .file_stem()
        .unwrap_or_else(|| {
            panic!(
                "Could not retrieve filename without extension from path {}",
                filepath
            )
        })
        .to_string_owned_or_panic()
        .replace("\\", "/")
}

/// NOTE: This also creates all necessary folders
pub fn path_copy_file(from_filepath: &str, to_filepath: &str) {
    let to_create_path = path_without_filename(to_filepath);
    std::fs::create_dir_all(path_without_filename(to_filepath)).unwrap_or_else(|error| {
        panic!(
            "Failed to copy file from '{}' to '{}': Could not create necessary folder '{}': {}",
            from_filepath, to_filepath, to_create_path, error
        )
    });
    std::fs::copy(from_filepath, to_filepath).unwrap_or_else(|error| {
        panic!(
            "Failed to copy file from '{}' to '{}': {}",
            from_filepath, to_filepath, error
        )
    });
}

/// NOTE: This also creates all necessary folders including the `to_folderpath`
pub fn path_copy_directory_contents_recursive(from_folderpath: &str, to_folderpath: &str) {
    assert!(
        Path::new(from_folderpath).is_dir(),
        "Could not copy from '{}' to '{}': Source folder '{}' is not a folder",
        from_folderpath,
        to_folderpath,
        to_folderpath
    );
    let files_to_copy = collect_files_recursive(from_folderpath);
    for sourcepath in &files_to_copy {
        let destpath = sourcepath.replace(from_folderpath, to_folderpath);
        path_copy_file(sourcepath, &destpath);
    }
}

/// NOTE: Result is prefixed by the given `root_folder` and contains Unix-style file seperators only
pub fn collect_files_by_extension_recursive(root_folder: &str, extension: &str) -> Vec<String> {
    walkdir::WalkDir::new(root_folder)
        .into_iter()
        .filter_map(|maybe_entry| maybe_entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_borrowed_or_panic()
                .ends_with(extension)
        })
        .map(|entry| entry.path().to_string_owned_or_panic().replace("\\", "/"))
        .collect()
}

/// NOTE: Result is prefixed by the given `root_folder` and contains Unix-style file seperators only
pub fn collect_files_recursive(root_folder: &str) -> Vec<String> {
    walkdir::WalkDir::new(root_folder)
        .into_iter()
        .filter_map(|maybe_entry| maybe_entry.ok())
        .filter(|entry| !entry.path().is_dir())
        .map(|entry| entry.path().to_string_owned_or_panic().replace("\\", "/"))
        .collect()
}

/// Glob patterns: https://en.wikipedia.org/wiki/Glob_%28programming%29
/// NOTE: Result is prefixed by the given `root_folder` and contains Unix-style file seperators only
pub fn collect_files_by_glob_pattern(root_folder: &str, glob_pattern: &str) -> Vec<String> {
    let pattern = path_join(root_folder, glob_pattern);
    glob::glob(&pattern)
        .unwrap()
        .into_iter()
        .filter_map(|maybe_entry| maybe_entry.ok())
        .map(|entry| entry.to_string_owned_or_panic().replace("\\", "/"))
        .collect()
}

// Returns stdout and stderr
pub fn run_systemcommand_fail_on_error(command: &str, print_command: bool) -> easy_process::Output {
    let result = easy_process::run(command);
    if let Ok(output) = result {
        let result = format!(
            "> {}\nstdout: '{}'\nstderr: '{}'",
            command, output.stdout, output.stderr,
        );
        if print_command {
            println!("{}", result);
        }
        output
    } else {
        let error = result.unwrap_err();
        match error {
            easy_process::Error::Failure(error_status, output) => {
                panic!(
                    "Failed command:\n'{}'\nstatus: '{}'\nstdout: '{}'\nstderr: '{}'",
                    command, error_status, output.stdout, output.stderr
                );
            }
            easy_process::Error::Io(error) => {
                panic!("Unable to execute command:\n'{}'\n{}", command, error)
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Appdata

fn get_home_dir() -> Result<String, String> {
    directories::UserDirs::new()
        .ok_or("Could not find home directory".to_string())?
        .home_dir()
        .to_string_owned()
}

fn get_appdata_dir(company_name: &str, application_name: &str) -> Result<String, String> {
    let project_dirs = directories::ProjectDirs::from("", company_name, application_name)
        .ok_or("Could not get appdata dir - home directory not found".to_string())?;

    let appdata_dir_path = project_dirs.data_dir();
    if let Some(appdata_dir) = appdata_dir_path.to_str() {
        let appdata_dir = appdata_dir.replace("\\data", "");
        std::fs::create_dir_all(&appdata_dir)
            .map_err(|error| format!("Could not get appdata dir - {}", error))?;
        // NOTE: On Windows `data_dir()` returns "{RoamingAppData}\_project_path_\data"
        //       which is not what we want
        Ok(appdata_dir)
    } else {
        Err(format!(
            "Could not get appdata dir - path '{:?}' is invalid",
            appdata_dir_path
        ))
    }
}

#[cfg(target_os = "windows")]
pub fn get_user_savedata_dir(company_name: &str, application_name: &str) -> Result<String, String> {
    let TODO = "use a `internal_mode` feature here";
    /*
    // Try working dir first: Write test file to see if we even have writing permissions for './'
        if std::fs::write("test.txt", "test").is_ok() {
            if std::fs::remove_file("test.txt").is_ok() {
                return Ok("".to_owned());
            }
        }
    */

    // Check canonical savegame dir
    if let Ok(user_home_path) = get_home_dir() {
        let savegame_path =
            user_home_path + "\\Saved Games\\" + company_name + "\\" + application_name;
        if let Err(error) = std::fs::create_dir_all(&savegame_path) {
            log::info!(
                "Cannot create savegame directory at '{}' : {}",
                &savegame_path,
                error
            )
        } else {
            return Ok(savegame_path);
        }
    }

    get_appdata_dir(company_name, application_name)
}

#[cfg(not(target_os = "windows"))]
pub fn get_user_savedata_dir(company_name: &str, application_name: &str) -> Result<String, String> {
    let TODO = "use a `internal_mode` feature here";
    /*
    // Try working dir first: Write test file to see if we even have writing permissions for './'
        if std::fs::write("test.txt", "test").is_ok() {
            if std::fs::remove_file("test.txt").is_ok() {
                return Ok("".to_owned());
            }
        }
    */

    get_appdata_dir(company_name, application_name)
}
