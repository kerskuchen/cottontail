use ct_lib_core::platform::{self, PathHelper};
use ct_lib_core::serde_json;
use ct_lib_core::{indexmap::IndexMap, panic_set_hook_wait_for_keypress};

use chrono::prelude::*;
use heck::{CamelCase, TitleCase};

type ProjectDetails = IndexMap<String, String>;
type ProjectDetailsLocal = IndexMap<String, String>;
type ProjectDetailsMerged = IndexMap<String, String>;

const PROGRAM_USAGE: &str = "Expected usage: 
 - for refreshing a project we can call `ct_makeproject` without any arguments.
 - for creating a new project we can use: `ct_makeproject <PROJECT_NAME> [PROJECT_GIT_URL]`";

fn create_default_project_details(project_name: String) -> (ProjectDetails, ProjectDetailsLocal) {
    let project_display_name = project_name.to_title_case();
    let project_company_name = "SnailSpaceGames".to_owned();
    let project_copyright_year = Utc::now().year().to_string();

    let windows_appdata_dir = project_display_name.to_camel_case();

    let mut details: ProjectDetails = IndexMap::new();
    details.insert("project_name".to_owned(), project_name);
    details.insert("project_display_name".to_owned(), project_display_name);
    details.insert(
        "project_company_name".to_owned(),
        project_company_name.clone(),
    );
    details.insert("project_copyright_year".to_owned(), project_copyright_year);
    details.insert("windows_appdata_dir".to_owned(), windows_appdata_dir);

    let mut details_local: ProjectDetailsLocal = IndexMap::new();
    details_local.insert("windows_certificate_path".to_owned(), "".to_owned());

    (details, details_local)
}

fn get_or_generate_project_details(project_name: String) -> ProjectDetailsMerged {
    let (project_details_default, project_details_local_default) =
        create_default_project_details(project_name);

    let mut project_details = IndexMap::new();
    let mut project_details_local = IndexMap::new();

    // Read project details from our json files
    if platform::path_exists("project_details.json") {
        let metadata_string = std::fs::read_to_string("project_details.json").unwrap();
        project_details = serde_json::from_str(&metadata_string)
            .expect("Failed to parse project details from 'project_details.json'")
    };
    if platform::path_exists("project_details_local.json") {
        let metadata_string = std::fs::read_to_string("project_details_local.json").unwrap();
        project_details_local = serde_json::from_str(&metadata_string)
            .expect("Failed to parse project details from 'project_details_local.json'")
    };

    // Check if our loaded project details contain any non-default entries
    for key in project_details.keys() {
        if !project_details_default.contains_key(key) {
            panic!("'project_details.json' contains unknown key {}", key);
        }
    }
    for key in project_details_local.keys() {
        if !project_details_local_default.contains_key(key) {
            panic!("'project_details_local.json' contains unknown key {}", key);
        }
    }

    // Fill in new or missing entries
    for (key, value) in &project_details_default {
        if !project_details.contains_key(key) {
            project_details.insert(key.clone(), value.clone());
        }
    }
    for (key, value) in &project_details_local_default {
        if !project_details_local.contains_key(key) {
            project_details_local.insert(key.clone(), value.clone());
        }
    }

    // Write back updated project details
    std::fs::write(
        "project_details.json",
        serde_json::to_string_pretty(&project_details).unwrap(),
    )
    .expect("Failed to write project details to 'project_details.json'");
    std::fs::write(
        "project_details_local.json",
        serde_json::to_string_pretty(&project_details_local).unwrap(),
    )
    .expect("Failed to write project details to 'project_details_local.json'");

    project_details.extend(project_details_local);
    project_details
}

fn template_text_as_comment(text: &str, file_extension: &str) -> String {
    if file_extension == "bat" {
        return "@echo off\r\nREM ".to_owned() + text + "\r\n\r\n";
    } else if file_extension == "java"
        || file_extension == "json"
        || file_extension == "cc"
        || file_extension == "hh"
        || file_extension == "gradle"
        || file_extension == "rs"
    {
        return "// ".to_owned() + text + "\n\n";
    } else if file_extension == "vcxproj"
        || file_extension == "user"
        || file_extension == "sln"
        || file_extension == "props"
    {
        // We don't allow comments in those files as it leads to file corruption
        return "".to_owned();
    } else {
        return "# ".to_owned() + text + "\n\n";
    }
}

/// Renders a given mustache template file and writes it to a given file using provided template values
fn template_copy(
    template_filepath: &str,
    output_filepath: &str,
    template_values: &ProjectDetailsMerged,
    generate_warning_header: bool,
) {
    let template = mustache::compile_path(template_filepath).expect(&format!(
        "Could not load template file '{}'",
        template_filepath,
    ));
    let rendered_template = template.render_to_string(&template_values).expect(&format!(
        "Failed to render template file '{}'",
        template_filepath,
    ));

    let file_extension = platform::path_to_extension(template_filepath);
    let warning_message = template_text_as_comment(
        &("WARNING: This file was generated from '".to_owned()
            + template_filepath
            + "' and should not be modified."),
        &file_extension,
    );

    let output_dir = platform::path_without_filename(output_filepath);
    std::fs::create_dir_all(&output_dir).expect(&format!("Could not create path {}", &output_dir));

    let final_rendered_template = if generate_warning_header {
        warning_message + &rendered_template
    } else {
        rendered_template
    };

    std::fs::write(output_filepath, final_rendered_template).expect(&format!(
        "Could not write template '{}' to '{}'",
        template_filepath, output_filepath
    ));
}

/// Renders a given mustache template file and writes its to a given directory using provided
/// template values.
/// NOTE: The `template__` file prefix will be removed automatically when writing out the file
fn template_copy_to_dir(
    template_filepath: &str,
    output_dir: &str,
    template_values: &ProjectDetailsMerged,
    generate_warning_header: bool,
) {
    let output_filename = platform::path_to_filename(template_filepath).replace("template__", "");
    let output_filepath = platform::path_join(output_dir, &output_filename);
    template_copy(
        template_filepath,
        &output_filepath,
        template_values,
        generate_warning_header,
    );
}

fn project_refresh() {
    // Get project details
    let project_name = {
        assert!(
            platform::path_exists("cottontail/ct_makeproject"),
            "{}\n{}",
            "ct_makeproject without any arguments must be run in the project root which contains the Cottontail library",
            PROGRAM_USAGE
        );
        let current_working_dir =
            std::env::current_dir().expect("Cannot determine current working directory");

        let working_dir = current_working_dir
            .file_name()
            .unwrap()
            .to_string_owned_or_panic();
        if working_dir.starts_with("ct_") {
            // NOTE: We want to reserve the `ct_` folder-name-prefix for ourselves
            working_dir.replacen("ct_", "", 1)
        } else {
            working_dir
        }
    };
    let project_details = get_or_generate_project_details(project_name.clone());

    // ---------------------------------------------------------------------------------------------
    // Repository setup

    for template_filepath in &[
        "cottontail/ct_makeproject/templates_repository/template__.gitattributes",
        "cottontail/ct_makeproject/templates_repository/template__.gitignore",
        "cottontail/ct_makeproject/templates_repository/template__git_update_cottontail.bat",
        "cottontail/ct_makeproject/templates_repository/template__project_refresh.bat",
    ] {
        template_copy_to_dir(template_filepath, "./", &project_details, true);
    }
    if !platform::path_exists("LICENSE.txt") {
        template_copy_to_dir(
            "cottontail/ct_makeproject/templates_repository/template__LICENSE.txt",
            "./",
            &project_details,
            false,
        );
    }
    if !platform::path_exists("Cargo.toml") {
        template_copy_to_dir(
            "cottontail/ct_makeproject/templates_repository/template__Cargo.toml",
            "./",
            &project_details,
            false,
        );
    }
    if !platform::path_exists("README.md") || std::fs::metadata("README.md").unwrap().len() == 0 {
        template_copy_to_dir(
            "cottontail/ct_makeproject/templates_repository/template__README.md",
            "./",
            &project_details,
            false,
        );
    }

    // ---------------------------------------------------------------------------------------------
    // Executable setup

    if !platform::path_exists("launcher/Cargo.toml") {
        template_copy_to_dir(
            "cottontail/ct_makeproject/templates_executable/template__Cargo.toml",
            "launcher",
            &project_details,
            false,
        );
    }
    if !platform::path_exists("launcher/main_launcher.rs") {
        template_copy_to_dir(
            "cottontail/ct_makeproject/templates_executable/template__main_launcher.rs",
            "launcher/src",
            &project_details,
            false,
        );
    }
    // NOTE: This should overwrite always
    template_copy_to_dir(
        "cottontail/ct_makeproject/templates_executable/template__main_launcher_info.rs",
        "launcher/src",
        &project_details,
        false,
    );

    // ---------------------------------------------------------------------------------------------
    // Assets setup

    for template_filepath in
        &["cottontail/ct_makeproject/templates_assets/template__assets_autobake.bat"]
    {
        template_copy_to_dir(template_filepath, "./", &project_details, true);
    }
    if !platform::path_exists("assets") {
        for template_filepath in
            &["cottontail/ct_makeproject/templates_assets/template__credits.txt"]
        {
            template_copy_to_dir(template_filepath, "assets", &project_details, false);
        }
        platform::path_copy_directory_contents_recursive(
            "cottontail/ct_makeproject/templates_assets/assets",
            "assets",
        );
    }
    if !platform::path_exists("assets_copy") {
        platform::path_copy_directory_contents_recursive(
            "cottontail/ct_makeproject/templates_assets/assets_copy",
            "assets_copy",
        );
    }
    if !platform::path_exists("assets_executable") {
        platform::path_copy_directory_contents_recursive(
            "cottontail/ct_makeproject/templates_assets/assets_executable",
            "assets_executable",
        );
    }

    // ---------------------------------------------------------------------------------------------
    // Windows project setup

    template_copy_to_dir(
        "cottontail/ct_makeproject/templates_windows/template__versioninfo.rc",
        "./assets_executable/",
        &project_details,
        false,
    );
    template_copy_to_dir(
        "cottontail/ct_makeproject/templates_windows/template__windows_build_shipping.bat",
        "./",
        &project_details,
        true,
    );
    if !project_details
        .get("windows_certificate_path")
        .as_ref()
        .unwrap()
        .is_empty()
    {
        template_copy_to_dir(
            "cottontail/ct_makeproject/templates_windows/template__windows_sign_executable.bat",
            "./",
            &project_details,
            true,
        );
    }

    // ---------------------------------------------------------------------------------------------
    // VSCode setup

    template_copy_to_dir(
        "cottontail/ct_makeproject/templates_vscode/template__tasks.json",
        "./.vscode/",
        &project_details,
        true,
    );
    template_copy_to_dir(
        "cottontail/ct_makeproject/templates_vscode/template__launch.json",
        "./.vscode/",
        &project_details,
        true,
    );

    println!("FINISHED REFRESHING PROJECT INFO");
}

fn project_create(project_directory_name: &str, project_git_url: Option<String>) {
    // Save the current working dir for later
    let start_working_dir =
        std::env::current_dir().expect("Cannot determine current working directory");

    // Create project dir
    assert!(
        !platform::path_exists(project_directory_name),
        "A directory with the name '{}' already exists",
        project_directory_name
    );
    std::fs::create_dir(&project_directory_name).expect("Cannot create project directory");
    std::env::set_current_dir(&std::path::Path::new(&project_directory_name))
        .expect("Cannot switch to project directory");

    // Init git repo and add initial commit
    std::fs::write("README.md", "").expect("Cannot create readme file");
    for command in &[
        "git init",
        "git add README.md",
        "git commit -am \"Initial commit\"",
    ] {
        print!(
            "> {}\n{}",
            command,
            platform::easy_process::run(command)
                .expect("Cannot make initial commit")
                .stdout
        );
    }

    // Add Cottontail as git submodule
    for command in &[
        "git submodule add -b master https://github.com/kerskuchen/cottontail.git",
        "git submodule update --init --remote",
        "git commit -am \"Adds Cottontail submodule\"",
    ] {
        print!(
            "> {}\n{}",
            command,
            platform::easy_process::run(command)
                .expect(&format!("Failed to add Cottontail"))
                .stdout
        );
    }

    // Connect our repo to its remote and do an initial push
    if let Some(project_url) = project_git_url {
        for command in &[
            "git remote add origin ".to_owned() + &project_url,
            "git push -u origin master".to_owned(),
        ] {
            print!(
                "> {}\n{}",
                command,
                platform::easy_process::run(command)
                    .expect(&format!(
                        "Initial push to remote '{}' failed (does it exist?)",
                        &project_url,
                    ))
                    .stdout
            );
        }
    }

    project_refresh();

    // Restore previous working dir
    std::env::set_current_dir(start_working_dir)
        .expect("Cannot switch back to initial working directory");
    println!(
        "FINISHED PROJECT INITIALIZATION '{}'",
        project_directory_name
    );
}

fn main() {
    panic_set_hook_wait_for_keypress();

    let (project_name, project_git_url) = {
        let args: Vec<String> = std::env::args().collect();
        assert!(args.len() <= 3, PROGRAM_USAGE);

        if args.len() == 3 {
            (Some(args[1].clone()), Some(args[2].clone()))
        } else if args.len() == 2 {
            (Some(args[1].clone()), None)
        } else {
            (None, None)
        }
    };

    if let Some(project_name) = project_name {
        project_create(&project_name, project_git_url);
    } else {
        project_refresh();
    }
}
