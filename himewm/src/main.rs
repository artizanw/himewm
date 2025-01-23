#![windows_subsystem = "windows"]

use windows::Win32::{

    UI::{

        Accessibility::*, 

        WindowsAndMessaging::*

    },

    System::Com::*

};

mod init;

mod tray_menu;

fn main() {

    // Maybe error handle this
    let _create_dirs = init::create_dirs();
    
    let user_settings = init::initialize_settings();

    let mut msg = MSG::default();

    unsafe {
        
        let layout_groups = match init::initialize_layouts() {

            Some(val) => val,
        
            None => {
                
                himewm::show_error_message("No layouts found");

                return;

            },
        
        };

        himewm::register_hotkeys();

        let _create_tray_icon = tray_menu::create();

        tray_menu::set_menu_event_handler();

        let mut wm = himewm::WindowManager::new(user_settings.to_settings(&layout_groups));

        wm.initialize(layout_groups.into_iter().map(|(_, layout)| layout).collect());

        while GetMessageA(&mut msg, None, 0, 0).as_bool() {

            himewm::handle_message(msg, &mut wm);

            let _translate_message = TranslateMessage(&msg);

            DispatchMessageA(&msg);

        }

        let _unhook_win_event = UnhookWinEvent(wm.event_hook);

        CoUninitialize();

    }

}
