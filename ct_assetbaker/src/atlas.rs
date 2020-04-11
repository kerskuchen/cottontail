use super::{AssetAtlas, SpritePosition, Spritename};

use ct_lib::math::*;
use ct_lib::system;

use indexmap::IndexMap;
use serde_derive::Deserialize;

pub fn atlas_create_from_pngs(
    source_dir: &str,
    output_dir: &str,
    atlas_texture_size: u32,
) -> AssetAtlas {
    let imagepaths = system::collect_files_by_extension_recursive(source_dir, ".png");

    // Our `texpack` tool needs our filelist as a text file
    let mut imagelist_string = String::from("");
    for imagepath in &imagepaths {
        imagelist_string += imagepath;
        imagelist_string += "\n";
    }
    let imagelist_path = system::path_join(output_dir, "imagelist.txt");
    std::fs::write(&imagelist_path, &imagelist_string).unwrap();

    let atlas_path_without_extension = system::path_join(output_dir, "atlas");

    let command = String::from("texpack")
        + " --trim"
        + " --size"
        + &format!(" {}x{}", atlas_texture_size, atlas_texture_size)
        + " --pretty"
        + " --premultiplied"
        + " --format \"jsonarray\""
        + " --output "
        + &atlas_path_without_extension
        + " "
        + &imagelist_path;
    system::run_systemcommand_fail_on_error(&command, false);

    let mut texture_imagepaths = system::collect_files_by_glob_pattern(&output_dir, "atlas*.png");
    let mut texture_metapaths = system::collect_files_by_glob_pattern(&output_dir, "atlas*.json");
    assert!(
        texture_imagepaths.len() == texture_metapaths.len()
            && texture_imagepaths.len() > 0
            && texture_metapaths.len() > 0,
        "Failed to generate sprite atlas '{}' - missing textures and/or metadata files",
        &atlas_path_without_extension,
    );

    // NOTE: If we only have one atlas texture we want it to have the same naming convention as
    //       if it were many
    if texture_imagepaths.len() == 1 {
        std::fs::rename(
            atlas_path_without_extension.clone() + ".png",
            atlas_path_without_extension.clone() + "-0.png",
        )
        .unwrap();
        std::fs::rename(
            atlas_path_without_extension.clone() + ".json",
            atlas_path_without_extension.clone() + "-0.json",
        )
        .unwrap();
        texture_imagepaths = vec![atlas_path_without_extension.clone() + "-0.png"];
        texture_metapaths = vec![atlas_path_without_extension.clone() + "-0.json"];
    }

    // Locate our sprites in each atlas texture
    let mut result_sprite_positions: IndexMap<Spritename, SpritePosition> = IndexMap::new();
    for texture_metapath in &texture_metapaths {
        let metadata_string = std::fs::read_to_string(&texture_metapath).unwrap();
        let meta: AtlasJSON = serde_json::from_str(&metadata_string).expect(&format!(
            "Failed to generate sprite atlas '{}' - Cannot parse metadata '{}'",
            &atlas_path_without_extension, texture_metapath
        ));

        let atlas_index = system::path_to_filename_without_extension(texture_metapath)
            .replace("atlas-", "")
            .parse::<u32>()
            .unwrap();
        for frame in meta.frames {
            result_sprite_positions.insert(
                frame.filename.replace(&format!("{}/", source_dir), ""),
                SpritePosition {
                    atlas_texture_index: atlas_index,
                    atlas_texture_pixel_offset: Vec2i::new(frame.frame.x, frame.frame.y),
                },
            );
        }
    }

    // As we are done with our metafiles we rename them so that the next time we run our texture
    // packer it does not get confused. We could of course just delete the files but they may be
    // useful for debug purposes
    for texture_metapath in &texture_metapaths {
        std::fs::rename(&texture_metapath, &(texture_metapath.clone() + ".backup")).unwrap();
    }

    AssetAtlas {
        texture_size: atlas_texture_size,
        texture_count: texture_imagepaths.len() as u32,
        texture_imagepaths,
        sprite_positions: result_sprite_positions,
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Generated JSON structs

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct AtlasJSON {
    frames: Vec<Frame>,
    meta: Meta,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct Frame {
    filename: String,
    frame: Frame2,
    trimmed: bool,
    #[serde(rename = "spriteSourceSize")]
    sprite_source_size: SpriteSourceSize,
    #[serde(rename = "sourceSize")]
    source_size: SourceSize,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct Frame2 {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct SpriteSourceSize {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct SourceSize {
    w: i32,
    h: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct Meta {
    app: String,
    image: String,
    size: Size,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct Size {
    w: i32,
    h: i32,
}
