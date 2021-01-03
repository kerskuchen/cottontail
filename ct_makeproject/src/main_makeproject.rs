use ct_lib_core::serde_json;
use ct_lib_core::*;
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
    if path_exists("project_details.json") {
        let metadata_string = std::fs::read_to_string("project_details.json").unwrap();
        project_details = serde_json::from_str(&metadata_string)
            .expect("Failed to parse project details from 'project_details.json'")
    };
    if path_exists("project_details_local.json") {
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

/// Renders a given mustache template file and writes it to a given file using provided template values
fn copy_template(
    template_filepath: &str,
    output_filepath: &str,
    template_values: &ProjectDetailsMerged,
) {
    if path_to_filename(template_filepath).starts_with("template#")
        || path_to_filename(template_filepath).starts_with("template_norefresh#")
    {
        println!("Template {} -> {}", template_filepath, output_filepath);

        let template = mustache::compile_path(template_filepath).expect(&format!(
            "Could not load template file '{}'",
            template_filepath,
        ));
        let rendered_template = template.render_to_string(&template_values).expect(&format!(
            "Failed to render template file '{}'",
            template_filepath,
        ));

        let output_dir = path_without_filename(output_filepath);
        std::fs::create_dir_all(&output_dir)
            .expect(&format!("Could not create path {}", &output_dir));

        std::fs::write(output_filepath, rendered_template).expect(&format!(
            "Could not write template '{}' to '{}'",
            template_filepath, output_filepath
        ));
    } else {
        // Regular file - just copy it
        println!("File {} -> {}", template_filepath, output_filepath);
        path_copy_file(template_filepath, output_filepath);
    }
}

fn refresh_or_copy_file_template_if_necessary(
    template_filepath: &str,
    root_source: &str,
    root_dest: &str,
    project_details: &IndexMap<String, String>,
) {
    let components: Vec<String> = template_filepath
        .replace(root_source, "")
        .split("/")
        .map(|component| component.to_owned())
        .collect();

    let mut output_filepath_accumulator = root_dest.to_owned();
    let mut original_filepath_accumulator = root_source.to_owned();
    for component in &components {
        original_filepath_accumulator = path_join(&original_filepath_accumulator, &component);
        output_filepath_accumulator = path_join(
            &output_filepath_accumulator,
            &component
                .replace("template_norefresh#", "")
                .replace("template#", ""),
        );

        if component.starts_with("template_norefresh#") {
            if path_exists(&output_filepath_accumulator) {
                // We don't copy this file because it already exists or one of its
                // parent directories exist
                return;
            }
        } else if component.starts_with("template#") {
            assert!(
                        path_is_file(&original_filepath_accumulator),
                        "'template#' can only be used for files (use 'template_norefresh#' for directories) - '{}' part in '{}'",
                        original_filepath_accumulator,
                        template_filepath
                    );
        }
    }
    copy_template(
        &template_filepath,
        &output_filepath_accumulator,
        &project_details,
    );
}

fn project_refresh() {
    // Get project details
    let project_name = {
        assert!(
            path_exists("cottontail/ct_makeproject"),
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

    let root_source = "./cottontail/ct_makeproject/project_template/";
    let root_dest = "./";
    for filepath in &collect_files_recursive(root_source) {
        refresh_or_copy_file_template_if_necessary(
            &filepath,
            root_source,
            root_dest,
            &project_details,
        );
    }

    println!("FINISHED REFRESHING PROJECT INFO");
}

fn project_create(project_directory_name: &str, project_git_url: Option<String>) {
    // Save the current working dir for later
    let start_working_dir =
        std::env::current_dir().expect("Cannot determine current working directory");

    // Create project dir
    assert!(
        !path_exists(project_directory_name),
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
            easy_process::run(command)
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
            easy_process::run(command)
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
                easy_process::run(command)
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
