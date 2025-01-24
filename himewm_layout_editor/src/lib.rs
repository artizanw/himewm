use enums::{Color, FrameType};
use fltk::*;
use fltk_theme::*;
use himewm_layout::*;
use prelude::{GroupExt, WidgetBase, WidgetExt};

pub struct LayoutEditorGUI {
    app: app::App,
    pub window: window::Window, //TODO: this doesnh't have to be public
}

impl LayoutEditorGUI {

    pub fn create() -> Self {
        
        let app = app::App::default();
        
        let widget_scheme = WidgetScheme::new(SchemeType::Fluent);

        widget_scheme.apply();
        
        return LayoutEditorGUI {
            app,
            window: create_window(),
        };

    }

    pub fn run(mut self) {

        self.window.show();

        self.app.run().unwrap();

    }

    // TODO: this doesn't have to be public either
    pub fn group_widget_from_layout_at(&self, layout: &Layout, idx: usize) -> group::Group {

        let w = self.window.w()/2;

        let h = self.window.h()/2;

        let group = group::Group::default().with_size(w, h);

        let zones = layout.get_zones_at(idx);

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
