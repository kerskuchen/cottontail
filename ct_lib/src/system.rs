use std::ffi::OsStr;
use std::path::Path;

pub use easy_process;

pub trait PathHelper {
    fn to_string_owned(&self) -> String;
    fn to_string_borrowed(&self) -> &str;
}

impl PathHelper for Path {
    fn to_string_owned(&self) -> String {
        self.to_str()
            .unwrap_or_else(|| panic!("Could not convert path to String {:?}", self))
            .to_owned()
    }

    fn to_string_borrowed(&self) -> &str {
        self.to_str()
            .unwrap_or_else(|| panic!("Could not convert path to &str {:?}", self))
    }
}

impl PathHelper for OsStr {
    fn to_string_owned(&self) -> String {
        self.to_str()
            .unwrap_or_else(|| panic!("Could not convert path to String {:?}", self))
            .to_owned()
    }

    fn to_string_borrowed(&self) -> &str {
        self.to_str()
            .unwrap_or_else(|| panic!("Could not convert path to &str {:?}", self))
    }
}

pub fn path_exists(path: &str) -> bool {
    Path::new(path).exists()
}

/// NOTE: Result contains Unix-style file seperators only
pub fn path_join(first: &str, second: &str) -> String {
    Path::new(first)
        .join(second)
        .to_string_owned()
        .replace("\\", "/")
}

/// NOTE: Result contains Unix-style file seperators only
pub fn path_with_extension(filepath: &str, new_extension: &str) -> String {
    Path::new(filepath)
        .with_extension(new_extension)
        .to_string_owned()
        .replace("\\", "/")
}

/// NOTE: Result contains Unix-style file seperators only
pub fn path_without_extension(filepath: &str) -> String {
    Path::new(filepath)
        .with_extension("")
        .to_string_owned()
        .replace("\\", "/")
}

/// NOTE: Result contains Unix-style file seperators only
pub fn path_without_filename(filepath: &str) -> String {
    Path::new(filepath)
        .with_file_name("")
        .to_string_owned()
        .replace("\\", "/")
}

/// NOTE: Result contains Unix-style file seperators only
pub fn path_to_extension(filepath: &str) -> String {
    Path::new(filepath)
        .extension()
        .unwrap_or_else(|| panic!("Could not retrieve filename from path {}", filepath))
        .to_string_owned()
        .replace("\\", "/")
}

/// NOTE: Result contains Unix-style file seperators only
pub fn path_to_filename(filepath: &str) -> String {
    Path::new(filepath)
        .file_name()
        .unwrap_or_else(|| panic!("Could not retrieve filename from path {}", filepath))
        .to_string_owned()
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
        .to_string_owned()
        .replace("\\", "/")
}

/// NOTE: This also creates all necessary folders
pub fn path_copy_file(from_filepath: &str, to_filepath: &str) {
    let to_create_path = path_without_filename(to_filepath);
    std::fs::create_dir_all(path_without_filename(to_filepath)).expect(&format!(
        "Failed to copy file from '{}' to '{}': Could not create necessary folder '{}'",
        from_filepath, to_filepath, to_create_path,
    ));
    std::fs::copy(from_filepath, to_filepath).expect(&format!(
        "Failed to copy file from '{}' to '{}'",
        from_filepath, to_filepath
    ));
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
        .filter(|entry| entry.file_name().to_string_borrowed().ends_with(extension))
        .map(|entry| entry.path().to_string_owned().replace("\\", "/"))
        .collect()
}

/// NOTE: Result is prefixed by the given `root_folder` and contains Unix-style file seperators only
pub fn collect_files_recursive(root_folder: &str) -> Vec<String> {
    walkdir::WalkDir::new(root_folder)
        .into_iter()
        .filter_map(|maybe_entry| maybe_entry.ok())
        .filter(|entry| !entry.path().is_dir())
        .map(|entry| entry.path().to_string_owned().replace("\\", "/"))
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
        .map(|entry| entry.to_string_owned().replace("\\", "/"))
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
