use fltk::*;
use fltk_theme::*;
use prelude::{GroupExt, WidgetBase, WidgetExt};

pub struct LayoutEditorGUI {
    app: app::App,
    main_window: window::Window,
}

impl LayoutEditorGUI {

    pub fn create() -> Self {
        
        let app = app::App::default();
        
        let widget_scheme = WidgetScheme::new(SchemeType::Fluent);

        widget_scheme.apply();
        
        return LayoutEditorGUI {
            app,
            main_window: main_window(),
        };

    }

    pub fn run(mut self) {

        self.main_window.show();

        self.app.run().unwrap();

    }

}

fn main_window() -> window::Window {

        let primary_screen = app::Screen::new(0).unwrap();

        let mut main_window = window::Window::new(
            primary_screen.w()/4, 
            primary_screen.h()/4, 
            primary_screen.w()/2, 
            primary_screen.h()/2, 
            "PLACEHOLDER");

        main_window.make_resizable(true);

        main_window.end();

        return main_window;

}
