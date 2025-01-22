#![windows_subsystem = "windows"]

use windows::Win32::{

    Foundation::*, 
    
    UI::{

        Accessibility::*,

        Input::KeyboardAndMouse::*, 

        WindowsAndMessaging::*

    },

    System::{

        Com::*,

        Console::*

    }

};

mod init;

mod tray_menu;

mod hotkey_identifiers {

    pub const FOCUS_PREVIOUS: usize = 0;

    pub const FOCUS_NEXT: usize = 1;

    pub const SWAP_PREVIOUS: usize = 2;

    pub const SWAP_NEXT: usize = 3;

    pub const VARIANT_PREVIOUS: usize = 4;

    pub const VARIANT_NEXT: usize = 5;

    pub const LAYOUT_PREVIOUS: usize = 6;

    pub const LAYOUT_NEXT: usize = 7;

    pub const FOCUS_PREVIOUS_MONITOR: usize = 8;

    pub const FOCUS_NEXT_MONITOR: usize = 9;

    pub const SWAP_PREVIOUS_MONITOR: usize = 10;

    pub const SWAP_NEXT_MONITOR: usize = 11;

    pub const GRAB_WINDOW: usize = 12;

    pub const RELEASE_WINDOW: usize = 13;

    pub const REFRESH_WORKSPACE: usize = 14;

    pub const TOGGLE_WORKSPACE: usize = 15;

}

unsafe fn register_hotkeys() {
    
    let _focus_previous = RegisterHotKey(None, hotkey_identifiers::FOCUS_PREVIOUS as i32, MOD_ALT, 0x4A);

    let _focus_next = RegisterHotKey(None, hotkey_identifiers::FOCUS_NEXT as i32, MOD_ALT, 0x4B);

    let _swap_previous = RegisterHotKey(None, hotkey_identifiers::SWAP_PREVIOUS as i32, MOD_ALT, 0x48);

    let _swap_next = RegisterHotKey(None, hotkey_identifiers::SWAP_NEXT as i32, MOD_ALT, 0x4C);
    
    let _variant_previous = RegisterHotKey(None, hotkey_identifiers::VARIANT_PREVIOUS as i32, MOD_ALT | MOD_SHIFT, 0x4A);

    let _variant_next = RegisterHotKey(None, hotkey_identifiers::VARIANT_NEXT as i32, MOD_ALT | MOD_SHIFT, 0x4B);

    let _layout_previous = RegisterHotKey(None, hotkey_identifiers::LAYOUT_PREVIOUS as i32, MOD_ALT | MOD_SHIFT, 0x48);

    let _layout_next = RegisterHotKey(None, hotkey_identifiers::LAYOUT_NEXT as i32, MOD_ALT | MOD_SHIFT, 0x4C);

    let _focus_previous_monitor = RegisterHotKey(None, hotkey_identifiers::FOCUS_PREVIOUS_MONITOR as i32, MOD_ALT, 0x55);

    let _focus_next_monitor = RegisterHotKey(None, hotkey_identifiers::FOCUS_NEXT_MONITOR as i32, MOD_ALT, 0x49);

    let _swap_previous_monitor = RegisterHotKey(None, hotkey_identifiers::SWAP_PREVIOUS_MONITOR as i32, MOD_ALT, 0x59);

    let _swap_next_monitor = RegisterHotKey(None, hotkey_identifiers::SWAP_NEXT_MONITOR as i32, MOD_ALT, 0x4F);
    
    let _grab_window = RegisterHotKey(None, hotkey_identifiers::GRAB_WINDOW as i32, MOD_ALT | MOD_SHIFT | MOD_NOREPEAT, 0x55);

    let _release_window = RegisterHotKey(None, hotkey_identifiers::RELEASE_WINDOW as i32, MOD_ALT | MOD_SHIFT | MOD_NOREPEAT, 0x49);

    let _refresh_workspace = RegisterHotKey(None, hotkey_identifiers::REFRESH_WORKSPACE as i32, MOD_ALT | MOD_SHIFT | MOD_NOREPEAT, 0x59);

    let _toggle_workspace = RegisterHotKey(None, hotkey_identifiers::TOGGLE_WORKSPACE as i32, MOD_ALT | MOD_SHIFT | MOD_NOREPEAT, 0x4F);

}

unsafe fn handle_message(msg: MSG, wm: &mut himewm::WindowManager) {

    match msg.message {

        himewm::messages::WINDOW_CREATED => {

            wm.window_created(HWND(msg.wParam.0 as *mut core::ffi::c_void));

        },

        himewm::messages::WINDOW_DESTROYED => {

            wm.window_destroyed(HWND(msg.wParam.0 as *mut core::ffi::c_void));

        },

        himewm::messages::WINDOW_MINIMIZED_OR_MAXIMIZED => {

            wm.window_minimized_or_maximized(HWND(msg.wParam.0 as *mut core::ffi::c_void));

        },

        himewm::messages::WINDOW_CLOAKED => {

            wm.window_cloaked(HWND(msg.wParam.0 as *mut core::ffi::c_void));

        },

        himewm::messages::FOREGROUND_WINDOW_CHANGED => {

            wm.foreground_window_changed(HWND(msg.wParam.0 as *mut core::ffi::c_void));

        },

        himewm::messages::WINDOW_MOVE_FINISHED => {

            wm.window_move_finished(HWND(msg.wParam.0 as *mut core::ffi::c_void));

        },

        WM_HOTKEY => {

            match msg.wParam.0 {
                
                hotkey_identifiers::FOCUS_PREVIOUS => {

                    wm.focus_previous();

                },

                hotkey_identifiers::FOCUS_NEXT => {

                    wm.focus_next();

                },

                hotkey_identifiers::SWAP_PREVIOUS => {

                    wm.swap_previous();

                },

                hotkey_identifiers::SWAP_NEXT => {

                    wm.swap_next();

                },

                hotkey_identifiers::VARIANT_PREVIOUS => {

                    wm.variant_previous();

                },

                hotkey_identifiers::VARIANT_NEXT => {

                    wm.variant_next();

                },

                hotkey_identifiers::LAYOUT_PREVIOUS => {

                    wm.layout_previous();

                },

                hotkey_identifiers::LAYOUT_NEXT => {

                    wm.layout_next();

                },

                hotkey_identifiers::FOCUS_PREVIOUS_MONITOR => {

                    wm.focus_previous_monitor();

                },

                hotkey_identifiers::FOCUS_NEXT_MONITOR => {

                    wm.focus_next_monitor();

                },

                hotkey_identifiers::SWAP_PREVIOUS_MONITOR => {

                    wm.swap_previous_monitor();

                },

                hotkey_identifiers::SWAP_NEXT_MONITOR => {

                    wm.swap_next_monitor();

                },

                hotkey_identifiers::GRAB_WINDOW => {

                    wm.grab_window();

                },

                hotkey_identifiers::RELEASE_WINDOW => {

                    wm.release_window();

                },

                hotkey_identifiers::REFRESH_WORKSPACE => {

                    wm.refresh_workspace();

                },

                hotkey_identifiers::TOGGLE_WORKSPACE => {

                    wm.toggle_workspace();

                },

                _ => (),

            }

        },

        _ => (),
    
    }

}

unsafe fn show_error_message(message: &str) {

    let _free_console = FreeConsole();

    let _alloc_console = AllocConsole();
    
    let handle = GetStdHandle(STD_INPUT_HANDLE).unwrap();
    
    let mut console_mode = CONSOLE_MODE::default();
    
    let _get_console_mode = GetConsoleMode(handle, &mut console_mode);

    let _set_console_mode = SetConsoleMode(handle, console_mode & !ENABLE_ECHO_INPUT);

    println!("{}", message);
    println!("Press ENTER to exit");

    let mut buf = String::new();
    
    let _read_line = std::io::stdin().read_line(&mut buf);
    
}

fn main() {

    // Maybe error handle this
    let _create_dirs = init::create_dirs();
    
    let settings = init::initialize_settings();

    let mut msg = MSG::default();

    unsafe {
        
        let layout_groups = match init::initialize_layouts() {

            Some(val) => val,
        
            None => {
                
                show_error_message("No layouts found");

                return;

            },
        
        };

        register_hotkeys();

        let _create_tray_icon = tray_menu::create();

        tray_menu::set_menu_event_handler();

        let mut wm = himewm::WindowManager::new(settings);

        wm.initialize(layout_groups);

        while GetMessageA(&mut msg, None, 0, 0).as_bool() {

            handle_message(msg, &mut wm);

            let _translate_message = TranslateMessage(&msg);

            DispatchMessageA(&msg);

        }

        let _unhook_win_event = UnhookWinEvent(wm.event_hook);

        CoUninitialize();

    }

}
