use tray_icon::{

    menu::{

        Menu,
        
        MenuEvent,

        MenuId,

        MenuItemBuilder

    },

    TrayIcon,

    TrayIconBuilder

};

use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;

mod menu_ids {

    pub const QUIT: &str = "quit";

}

pub fn create() -> tray_icon::Result<TrayIcon> {

    let menu = Menu::new();

    let quit_item = MenuItemBuilder::new()
        .id(MenuId::new(menu_ids::QUIT))
        .text("Quit")
        .enabled(true)
        .build();

    menu.append(&quit_item).unwrap();

    return TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("himewm")
        .build();

}

pub unsafe fn set_menu_event_handler() {

    MenuEvent::set_event_handler(Some(|event: MenuEvent| {

        match event.id().as_ref() {
            
            menu_ids::QUIT => {

                PostQuitMessage(0);

            },

            _ => return,

        }


    }));

}
