use himewm_layout::*;

#[test]
fn show_layout() {

    let mut layout_group = LayoutGroup::new(1920, 1200);

    let idx = layout_group.default_idx();

    let variant = &mut layout_group.get_layouts_mut()[idx];

    variant.new_zone_vec();

    variant.split(1, 0, SplitDirection::Vertical(960));

    variant.new_zone_vec_from(1);

    variant.split(2, 1, SplitDirection::Horizontal(600));

    layout_group.new_variant_from(layout_group.default_idx());

    let new_variant = &mut layout_group.get_layouts_mut()[1];

    for _i in 0..2 {

        new_variant.delete_zones(1);

    }

    
    new_variant.new_zone_vec();

    new_variant.split(1, 0, SplitDirection::Vertical(960));

    new_variant.new_zone_vec();

    new_variant.split(2, 0, SplitDirection::Horizontal(600));
    new_variant.split(2, 1, SplitDirection::Vertical(960));

    new_variant.new_zone_vec_from(2);
    new_variant.split(3, 0, SplitDirection::Vertical(1280));

    new_variant.new_zone_vec_from(3);
    new_variant.split(4, 0, SplitDirection::Vertical(640));
    new_variant.swap_zones(4, 0, 2);
    let mut gui = himewm_layout_editor::LayoutEditorGUI::create();

    gui.edit_layout(layout_group);

    gui.run();

}
