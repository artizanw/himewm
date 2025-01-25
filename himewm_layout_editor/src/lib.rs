use std::clone;

use enums::{Color, FrameType};
use fltk::*;
use fltk_theme::*;
use group::ScrollType;
use himewm_layout::*;
use prelude::{GroupExt, WidgetBase, WidgetExt};

type ArcRwLock<T> = std::sync::Arc<std::sync::RwLock<T>>;

struct LayoutEditor {
    layout_group: LayoutGroup,
    selected_variant_idx: ArcRwLock<usize>,
    selected_zones_idx: ArcRwLock<usize>,
    selected_zone_idx1: Option<ArcRwLock<usize>>,
    selected_zone_idx2: Option<ArcRwLock<usize>>,
}

impl LayoutEditor {

    fn new(layout: LayoutGroup) -> Self {

        let default_idx = layout.default_idx();

        return LayoutEditor {
            layout_group: layout,
            selected_variant_idx: std::sync::Arc::new(std::sync::RwLock::new(default_idx)),
            selected_zones_idx: std::sync::Arc::new(std::sync::RwLock::new(0)),
            selected_zone_idx1: None,
            selected_zone_idx2: None,
        };

    }

}

struct EditorWidgets {
    editor: LayoutEditor,
    variant_list: group::Scroll,
    variant_buttons: Vec<button::Button>,
    zone_button_scrolls: group::Group,
    zone_button_scrolls_vector: Vec<group::Scroll>,
    zone_button_vectors: Vec<Vec<button::Button>>,
}

impl EditorWidgets {

    fn initialize(layout: LayoutGroup) -> Self {

        let editor = LayoutEditor::new(layout);

        let mut variant_list = group::Scroll::default_fill().with_type(ScrollType::Vertical);

        let mut variant_buttons = Vec::new();

        let mut zone_button_scrolls = group::Group::default_fill();

        zone_button_scrolls.end();

        let mut zone_button_scrolls_vector = Vec::new();

        let mut zone_button_vectors = Vec::new();

        let selected_variant_idx = editor.selected_variant_idx.clone();

        let selected_zones_idx = editor.selected_zones_idx.clone();

        variant_list.set_size(variant_list.w()/8, variant_list.h()/2);

        variant_list.set_color(Color::Background2);

        for (i, variant) in editor.layout_group.get_layouts().iter().enumerate() {
            
            variant_list.begin();

            let mut variant_button = button::Button::default_fill();

            if i == editor.layout_group.default_idx() {

                variant_button.set_label(format!("{i} (default)").as_str());

            }

            else {

                variant_button.set_label(i.to_string().as_str());

            }

            variant_button.set_label_size(16);
            
            variant_button.set_size(variant_button.w(), 20);

            variant_button.set_color(Color::Background2);

            variant_button.set_selection_color(Color::Blue);

            let selected_variant_idx_clone = selected_variant_idx.clone();

            variant_button.set_callback(move |_| {

                *selected_variant_idx_clone.try_write().unwrap() = i;

            });

            variant_buttons.push(variant_button);

            variant_list.end();

            let mut zone_button_scroll = group::Scroll::default_fill().with_type(ScrollType::Horizontal);

            let new_height = 72 + zone_button_scroll.hscrollbar().h();

            zone_button_scroll.set_size(zone_button_scroll.w(), new_height);

            let mut zone_button_vector = Vec::new();

            for (j, zones) in variant.get_zones().iter().enumerate() {

                let mut zone_button = button::Button::default().with_size(64, 64).with_label(j.to_string().as_str()).center_y(&zone_button_scroll);

                zone_button.set_pos(j as i32*64 + 4, zone_button.y() - zone_button_scroll.hscrollbar().h());

                zone_button.set_label_size(32);

                let selected_zones_idx_clone = selected_zones_idx.clone();

                zone_button.set_callback(move |_| {

                    *selected_zones_idx_clone.try_write().unwrap() = j;

                });

                zone_button_vector.push(zone_button);

            }

            zone_button_scroll.end();

            zone_button_scrolls.add(&zone_button_scroll);

            zone_button_scrolls_vector.push(zone_button_scroll);

            zone_button_vectors.push(zone_button_vector);

        }

        return EditorWidgets {
            editor,
            variant_list,
            variant_buttons,
            zone_button_scrolls,
            zone_button_scrolls_vector,
            zone_button_vectors,
        };

    }
    
}

pub struct LayoutEditorGUI {
    app: app::App,
    window: window::Window,
    editor_widgets: Option<EditorWidgets>
}

impl LayoutEditorGUI {

    pub fn create() -> Self {
        
        initialize_colours();

        let app = app::App::default();
        
        let widget_scheme = WidgetScheme::new(SchemeType::Fluent);

        widget_scheme.apply();
        
        return LayoutEditorGUI {
            app,
            window: create_window(),
            editor_widgets: None,
        };

    }

    pub fn run(mut self) {

        self.window.show();

        self.app.run().unwrap();

    }

    fn edit_layout(&mut self, layout: LayoutGroup) {



    }

}
    
fn initialize_colours() {

    app::background(16, 16, 16);

    app::background2(32, 32, 32);

}

fn create_window() -> window::Window {

        let primary_screen = app::Screen::new(0).unwrap();

        let mut window = window::Window::new(
            primary_screen.w()/4, 
            primary_screen.h()/4, 
            primary_screen.w()/2, 
            primary_screen.h()/2, 
            "PLACEHOLDER");

        window.make_resizable(true);

        window.end();

        return window;

}

fn group_widget_from_layout_at(layout: &Layout, idx: usize) -> group::Group {

    let mut group = group::Group::default_fill();

    let w = group.w()/2;

    let h = group.h()/2;

    group.set_size(w, h);

    let zones = &layout.get_zones()[idx];

    let layout_width = layout.get_monitor_rect().right as f64;

    let layout_height = layout.get_monitor_rect().bottom as f64;

    for (i, zone) in zones.iter().enumerate() {

        let mut button = button::Button::new(
            ((zone.left as f64*w as f64)/layout_width).round() as i32, 
            ((zone.top as f64*h as f64)/layout_height).round() as i32, 
            ((zone.w() as f64*w as f64)/layout_width).round() as i32, 
            ((zone.h() as f64*h as f64)/layout_height).round() as i32, 
            Some(i.to_string().as_str())
        );

        // TODO: this frame type doesn't look too great - probably
        // figure out how to make it look better
        button.set_frame(FrameType::EmbossedBox);

        // TODO: set the actual colours
        button.set_color(Color::Blue);

        button.set_selection_color(Color::Magenta);

    }

    group.end();

    return group;

}
