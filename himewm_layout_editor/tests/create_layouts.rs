use directories::UserDirs;

use himewm_layout::*;

#[test]
fn create_vertical_stack() {

    let mut layout_group = LayoutGroup::new(1920, 1200);

    let mut idx = 0;

    let mut current_variant = &mut layout_group.get_layouts_mut()[idx];

    current_variant.new_zone_vec();

    for n in 1..=5 {

        current_variant.split(1, 0, SplitDirection::Vertical((1920 as f64*(n as f64/6 as f64)) as i32));

        if n < 3 {
            
            current_variant.swap_zones(1, 0, 1);

        }

        if n != 5 {

            layout_group.new_variant();

            idx += 1;

            current_variant = &mut layout_group.get_layouts_mut()[idx];

            current_variant.merge_zones(1, 0, 1);

        }

    }

    layout_group.set_default_idx(2);


    export_layout_to_downloads(&layout_group, "vertical_stack").unwrap();



}

fn export_layout_to_downloads(layout: &LayoutGroup, name: &str) -> std::io::Result<()> {
    
    let path = UserDirs::new().unwrap().download_dir().unwrap().join(std::path::Path::new(name).with_extension("json"));
    
    let output_file = std::fs::File::create_new(path)?;

    serde_json::to_writer_pretty(output_file, layout).unwrap();

    return Ok(());




}