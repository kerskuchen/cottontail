use ct_lib_core as core;
use ct_lib_image as image;

use crate::core::indexmap::IndexMap;
use crate::core::*;

use image::*;

use std::{collections::HashMap, fs::File, io::Write, path::Path};

pub const VERSION_INFO_TEMPLATE: &[u8] = include_bytes!("../resources/versioninfo.rc");

////////////////////////////////////////////////////////////////////////////////////////////////////
// Main

fn main() {
    let start_time = std::time::Instant::now();

    init_logging("target/packager_log.txt", log::Level::Trace).expect("Unable to init logging");
    std::panic::set_hook(Box::new(|panic_info| {
        let (message, location) = core::panic_message_split_to_message_and_location(panic_info);
        let final_message = format!("{}\n\nError occured at: {}", message, location);
        log::error!("{}", final_message);

        // NOTE: This forces the other threads to shutdown as well
        std::process::abort();
    }));

    let project_details = load_project_details();
    let windows_executable_output_path =
        format!("shipping_windows/{}.exe", project_details["project_name"]);

    // Build project in release mode
    path_recreate_directory_looped("shipping_windows");
    run_systemcommand_fail_on_error("cargo build --release --package launcher", false);
    run_systemcommand_fail_on_error("cargo run --package ct_assetbaker", false);
    if path_exists("resources") {
        path_copy_directory_contents_recursive("resources", "shipping_windows/resources");
    }

    // Check if resource hacker exists
    let resource_hacker_in_path = run_systemcommand("where ResourceHacker.exe", false).is_ok();
    if !resource_hacker_in_path {
        log::warn!("`ResourceHacker.exe` not detected in PATH - Skipping embedding launcher icon and version info");
    }

    // Process icons and executable resource info
    if resource_hacker_in_path
        && path_exists("assets_executable")
        && !path_dir_empty("assets_executable")
    {
        path_recreate_directory_looped("target/temp_executable");
        create_versioninfo(&project_details);

        // Create launcher icon
        let existing_launcher_icons = load_existing_launcher_icon_images("assets_executable");
        let windows_icon_images = create_windows_launcher_icon_images(&existing_launcher_icons);
        for (&size, image) in windows_icon_images.iter() {
            Bitmap::write_to_png_file(
                image,
                &format!("target/temp_executable/launcher_icons_windows/{}.png", size),
            );
        }
        create_windows_launcher_icon(&windows_icon_images, "target/temp_executable/launcher.ico");

        run_systemcommand_fail_on_error("ResourceHacker.exe -log target/temp_executable/log1.txt -open target/temp_executable/versioninfo.rc -save target/temp_executable/versioninfo.res -action compile",false);
        run_systemcommand_fail_on_error("ResourceHacker.exe -log target/temp_executable/log2.txt -open target/release/launcher.exe -save target/temp_executable/launcher_tmp1.exe -action add -res target/temp_executable/versioninfo.res", false);
        run_systemcommand_fail_on_error("ResourceHacker.exe -log target/temp_executable/log3.txt -open target/temp_executable/launcher_tmp1.exe -save target/temp_executable/launcher_tmp2.exe -action add -res target/temp_executable/launcher.ico -mask ICONGROUP,MAINICON,", false);
        path_copy_file(
            "target/temp_executable/launcher_tmp2.exe",
            &windows_executable_output_path,
        );
    } else {
        path_copy_file(
            "./target/release/launcher.exe",
            &windows_executable_output_path,
        );
    }

    create_zipfile(&project_details);

    log::info!(
        "PACKAGER FINISHED SUCCESSFULLY: Elapsed time: {:.3}s",
        start_time.elapsed().as_secs_f64()
    );
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Version info

fn create_versioninfo(project_details: &IndexMap<String, String>) {
    assert!(
        path_exists("project_details.json"),
        "Missing project_details.json - please re-run `ct_makeproject`"
    );

    // Render and write out our versioninfo.rc file
    let version_info_template =
        mustache::compile_str(&String::from_utf8(VERSION_INFO_TEMPLATE.to_vec()).unwrap()).unwrap();
    let version_info = version_info_template
        .render_to_string(&project_details)
        .unwrap();
    std::fs::write("target/temp_executable/versioninfo.rc", version_info)
        .expect("Could not write to 'target/temp_executable/versioninfo.rc'");
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Launcher icon

fn load_existing_launcher_icon_images(search_dir: &str) -> HashMap<i32, Bitmap> {
    let mut result = HashMap::new();
    let image_paths = collect_files_by_extension_recursive(search_dir, ".png");
    for image_path in &image_paths {
        let image = Bitmap::from_png_file_or_panic(image_path);
        let size = path_to_filename_without_extension(image_path)
            .replace("launcher_icon_", "")
            .parse()
            .expect(&format!(
                "Launcher icon name '{}' invalid, see README_ICONS.md",
                image_path,
            ));

        assert!(
            image.width == size && image.height == size,
            "Launcher icon name '{}' does not match its dimension ({}x{}), see README_ICONS.md",
            image_path,
            image.width,
            image.height
        );
        result.insert(size, image);
    }
    assert!(
        !result.is_empty(),
        "No launcher icons found at '{}'",
        search_dir
    );
    result
}

fn create_windows_launcher_icon_images(
    existing_launcher_icons: &HashMap<i32, Bitmap>,
) -> HashMap<i32, Bitmap> {
    let biggest_size = existing_launcher_icons.keys().max().unwrap();
    let windows_icon_sizes = [256, 128, 64, 48, 32, 16];
    let mut result = HashMap::new();
    for &size in windows_icon_sizes.iter() {
        if !existing_launcher_icons.contains_key(&size) {
            let scaled_image = existing_launcher_icons
                .get(&biggest_size)
                .unwrap()
                .scaled_sample_nearest_neighbor(size as u32, size as u32);
            result.insert(size, scaled_image);
        } else {
            let image = existing_launcher_icons.get(&size).unwrap();
            result.insert(size, image.clone());
        }
    }
    result
}

fn create_windows_launcher_icon(
    windows_icon_images: &HashMap<i32, Bitmap>,
    icon_output_filepath: &str,
) {
    let mut iconpacker = ico::IconDir::new(ico::ResourceType::Icon);
    for (_size, image) in windows_icon_images.iter() {
        let icon_image = ico::IconImage::from_rgba_data(
            image.width as u32,
            image.height as u32,
            image.to_bytes(),
        );

        iconpacker.add_entry(ico::IconDirEntry::encode(&icon_image).expect(&format!(
            "Cannot encode icon ({}x{}) into launcher icon",
            image.width, image.height,
        )));
    }
    let output_file = std::fs::File::create(icon_output_filepath)
        .expect(&format!("Could not create path '{}'", icon_output_filepath));
    iconpacker
        .write(output_file)
        .expect(&format!("Could not write to '{}'", icon_output_filepath));
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Misc

fn create_zipfile(project_details: &IndexMap<String, String>) {
    let files_to_zip = collect_files_recursive("shipping_windows");
    let dirs_to_zip = collect_directories_recursive("shipping_windows");
    let mut zip = {
        let zipfile_path = format!(
            "shipping_windows/{}_v{}.zip",
            project_details["project_name"], project_details["project_version"]
        );
        let writer = File::create(&zipfile_path)
            .expect(&format!("Cannot create output file {}", zipfile_path));
        zip::ZipWriter::new(writer)
    };
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    // Add dirs to zipfile
    for path in dirs_to_zip {
        let name = path
            .replace("shipping_windows/", "")
            .replace("shipping_windows", "");
        if !name.is_empty() {
            println!("adding dir {:?} as {:?} ...", path, name);
            #[allow(deprecated)]
            zip.add_directory_from_path(Path::new(&name), options)
                .expect(&format!(
                    "Cannot add directory {:?} to zip as {:?}",
                    path, name
                ));
        }
    }

    // Add files to zipfile
    for path in files_to_zip.into_iter() {
        let name = path.replace("shipping_windows/", "");

        log::debug!("adding file {:?} as {:?} ...", &path, name);
        let buffer = std::fs::read(&path).expect(&format!("Cannot read file {:?}", &path));

        #[allow(deprecated)]
        zip.start_file_from_path(Path::new(&name), options)
            .expect(&format!("Cannot add file {:?} to zip as {:?}", path, name));
        zip.write_all(&buffer).expect(&format!(
            "Cannot write file {:?} to zip as {:?}",
            path, name
        ));
    }

    zip.finish().expect("Failed to finalize zip file");
}

fn load_project_details() -> IndexMap<String, String> {
    let mut project_details: IndexMap<String, String> = {
        let metadata_string = std::fs::read_to_string("project_details.json").unwrap();
        serde_json::from_str(&metadata_string)
            .expect("Failed to parse project details from 'project_details.json'")
    };
    assert!(
            project_details.contains_key("project_display_name"),
            "`project_details.json` missing key `project_display_name` - please re-run `ct_makeproject`"
        );
    assert!(
        project_details.contains_key("project_name"),
        "`project_details.json` missing key `project_name` - please re-run `ct_makeproject`"
    );
    assert!(
            project_details.contains_key("project_company_name"),
            "`project_details.json` missing key `project_company_name` - please re-run `ct_makeproject`"
        );
    assert!(
            project_details.contains_key("project_copyright_year"),
            "`project_details.json` missing key `project_copyright_year` - please re-run `ct_makeproject`"
        );

    // Find out project version
    assert!(path_exists("launcher/Cargo.toml"));
    let project_version = std::fs::read_to_string("launcher/Cargo.toml")
        .expect("Could not read 'launcher/Cargo.toml'")
        .lines()
        .filter(|line| line.starts_with("version=") || line.starts_with("version ="))
        .map(|line| line.split("=").last().unwrap().trim().replace("\"", ""))
        .next()
        .expect("'launcher/Cargo.toml' does not contain a `version` line")
        .to_owned();
    project_details.insert("project_version".to_owned(), project_version);

    project_details
}
