use std::unimplemented;

use aseprite_reader::Aseprite;

use super::*;

impl Aseprite {
    pub fn list_layer_names(&self) -> Vec<String> {
        self.layers.iter().map(|layer| layer.name.clone()).collect()
    }

    pub fn render_frame_premultiplied(
        &self,
        frame_index: usize,
        ignored_layers: &[String],
    ) -> Bitmap {
        let mut layer_indices: Vec<usize> = self
            .layers
            .iter()
            .filter(|layer| !ignored_layers.contains(&layer.name))
            .map(|layer| layer.layer_index)
            .collect();
        layer_indices.sort();

        let mut result = Bitmap::new_filled(
            self.header.width_in_pixels as u32,
            self.header.height_in_pixels as u32,
            PixelRGBA::transparent(),
        );
        for layer_index in layer_indices {
            let layer_bitmap = self.render_cel_premultiplied(frame_index, layer_index);
            let layer_blend_mode = self.layers[layer_index].blend_mode;
            let blend_mode = match self.layers[layer_index].blend_mode {
                aseprite_reader::LayerBlendMode::Normal => AlphaBlendMode::Normal,
                aseprite_reader::LayerBlendMode::Multiply => AlphaBlendMode::Multiply,
                _ => {
                    unimplemented!(
                        "Unimplemented blend mode {:?} in frame {} layer {}",
                        layer_blend_mode,
                        frame_index,
                        layer_index
                    )
                }
            };
            layer_bitmap.premultiplied_blit_to_alpha_blended(
                &mut result,
                Vec2i::zero(),
                true,
                blend_mode,
            );
        }
        // Debug out
        result
            .to_unpremultiplied_alpha()
            .write_to_png_file(&format!("frame {}.png", frame_index));

        result
    }
}

fn aseprite_get_offsets_for_layer(filepath: &str, layer_name: &str, out_offsets: &mut Vec<Vec2i>) {
    let aseprite = Aseprite::from_file(filepath).expect("Failed to list aseprite file layers");
    let framecount = out_offsets.len();
    assert!(framecount > 0);

    /*
       let command = String::from("aseprite")
           + " --batch"
           + " --list-layers"
           + " --list-tags"
           + " --layer"
           + " \""
           + layer_name
           + "\""
           + " --format \"json-array\""
           + " --trim"
           + " --ignore-empty "
           + image_filepath
           + " --sheet "
           + output_filepath_image
           + " --data "
           + output_filepath_meta;
       run_systemcommand_fail_on_error(&command, false);

       assert!(
           path_exists(&output_filepath_image),
           "Failed to generate offset information for '{}' - '{}' is missing",
           image_filepath,
           output_filepath_image
       );
       assert!(
           path_exists(&output_filepath_meta),
           "Failed to generate offset information for '{}' - '{}' is missing",
           image_filepath,
           output_filepath_meta
       );

       // We don't need the actual offset image as it is just a bunch of merged pixels. We do need to
       // rename the image though so it does not get the texture packer confused in a later stage
       std::fs::rename(
           &output_filepath_image,
           &(output_filepath_image.to_owned() + ".backup"),
       )
       .unwrap();

       let metadata_string = std::fs::read_to_string(output_filepath_meta).unwrap();
       if metadata_string.len() == 0 {
           // NOTE: Sometimes we get an empty json file for images without offsets
           return;
       }

       let meta: AsepriteJSON = serde_json::from_str(&metadata_string).expect(&format!(
           "Failed to generate offset information for '{}' - Cannot parse metadata '{}'",
           image_filepath, output_filepath_meta
       ));

       assert!(
           meta.frames.len() == 0 || meta.frames.len() == framecount,
           "Failed to generate offset information for '{}' - Offset points in layer '{}' need
               to be placed either on every frame or on none",
           image_filepath,
           layer_name
       );

       for (index, frame) in meta.frames.iter().enumerate() {
           out_offsets[index] = Vec2i::new(frame.sprite_source_size.x, frame.sprite_source_size.y);
       }
    */
    todo!()
}

pub fn run() -> Result<(), String> {
    let filepath = "D:/Creating/ballon/cottontail/ct_assetbaker/resources/sorcy_test.ase";
    let aseprite = Aseprite::from_file(filepath)?;

    let out = dformat_pretty!(&aseprite);
    std::fs::write("asepritedump.txt", out).unwrap();

    dbg!(aseprite.list_layer_names());
    for frame_index in 0..aseprite.frames.len() {
        aseprite.render_frame_premultiplied(
            frame_index,
            &["pivot".to_owned(), "attachment_2".to_owned()],
        );
    }

    Ok(())
}
