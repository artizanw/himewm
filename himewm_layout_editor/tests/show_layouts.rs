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

    variant.extend();

    layout_group.new_variant_from(layout_group.default_idx());

    let new_variant = &mut layout_group.get_layouts_mut()[1];

    new_variant.delete_zones(3);

    let mut gui = himewm_layout_editor::LayoutEditorGUI::create();

    gui.edit_layout(layout_group);

    gui.run();

}
