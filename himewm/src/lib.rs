use himewm_layout::*;

use windows::{

    core::*,

    Win32::{

        Foundation::*, 
        
        Graphics::{
        
            Dwm::*, Gdi::*
        
        }, 
        
        System::{

            Com::*,

            Console::*,

        },
        
        UI::{
    
            Accessibility::*, 
            
            HiDpi::*, 

            Input::KeyboardAndMouse::*,
            
            Shell::*, 
            
            WindowsAndMessaging::*
        
        }

    }

};

pub mod messages {

    use windows::Win32::UI::WindowsAndMessaging::WM_APP;
    
    pub const WINDOW_CREATED: u32 = WM_APP + 1;
    
    pub const WINDOW_RESTORED: u32 = WM_APP + 2;

    pub const WINDOW_DESTROYED: u32 = WM_APP + 3;
    
    pub const WINDOW_MINIMIZED_OR_MAXIMIZED: u32 = WM_APP + 4;
    
    pub const WINDOW_CLOAKED: u32 = WM_APP + 5;
    
    pub const FOREGROUND_WINDOW_CHANGED: u32 = WM_APP + 6;

    pub const WINDOW_MOVE_FINISHED: u32 = WM_APP + 7;

}

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

const CREATE_RETRIES: i32 = 100;

pub struct Settings {
    pub default_layout_idx: usize,
    pub window_padding: i32,
    pub edge_padding: i32,
    pub disable_rounding: bool,
    pub disable_unfocused_border: bool,
    pub focused_border_colour: COLORREF,
}

impl Default for Settings {

    fn default() -> Self {
    
        Settings {
            default_layout_idx: 0,
            window_padding: 0,
            edge_padding: 0,
            disable_rounding: false,
            disable_unfocused_border: false,
            focused_border_colour: COLORREF(0x00FFFFFF),
        }
    
    }

}

impl Settings {

    fn get_unfocused_border_colour(&self) -> COLORREF {

        if self.disable_unfocused_border {

            return COLORREF(DWMWA_COLOR_NONE);

        }

        else {

            return COLORREF(DWMWA_COLOR_DEFAULT);

        }

    }

}

#[derive(Clone)]
struct Workspace {
    layout_idx: usize,
    variant_idx: usize,
    managed_window_handles: Vec<HWND>,
}

impl Workspace {

    unsafe fn new(hwnd: HWND, layout_idx: usize, variant_idx: usize) -> Self {

        Workspace {
            layout_idx,
            variant_idx,
            managed_window_handles: vec![hwnd],
        }

    }

}

#[derive(Clone)]
struct WindowInfo {

    desktop_id: GUID,
    monitor_handle: HMONITOR,
    restored: bool,
    idx: usize,

}

impl WindowInfo {

    fn new(desktop_id: GUID, monitor_handle: HMONITOR, restored: bool, idx: usize) -> Self {

        WindowInfo {
            desktop_id,
            monitor_handle,
            restored,
            idx,
        }

    }

}

pub struct WindowManager {
    pub event_hook: HWINEVENTHOOK,
    virtual_desktop_manager: IVirtualDesktopManager,
    monitor_handles: Vec<HMONITOR>,
    window_info: std::collections::HashMap<*mut core::ffi::c_void, WindowInfo>,
    workspaces: std::collections::HashMap<(GUID, *mut core::ffi::c_void), Workspace>,
    layouts: std::collections::HashMap<*mut core::ffi::c_void, Vec<LayoutGroup>>,
    foreground_window: Option<HWND>,
    grabbed_window: Option<HWND>,
    ignored_combinations: std::collections::HashSet<(GUID, *mut core::ffi::c_void)>,
    ignored_hwnds: std::collections::HashSet<*mut core::ffi::c_void>,
    settings: Settings,
}

impl WindowManager {

    pub unsafe fn new(settings: Settings) -> Self {

        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        WindowManager {
            event_hook: SetWinEventHook(EVENT_MIN, EVENT_MAX, None, Some(Self::event_handler), 0, 0, WINEVENT_OUTOFCONTEXT),
            virtual_desktop_manager: CoCreateInstance(&VirtualDesktopManager, None, CLSCTX_INPROC_SERVER).unwrap(),
            monitor_handles: Vec::new(),
            window_info: std::collections::HashMap::new(),
            workspaces: std::collections::HashMap::new(),
            layouts: std::collections::HashMap::new(),
            foreground_window: None,
            grabbed_window: None,
            ignored_combinations: std::collections::HashSet::new(),
            ignored_hwnds: std::collections::HashSet::new(),
            settings,
        }
            
    }

    pub unsafe fn initialize(&mut self, layout_groups: Vec<LayoutGroup>) {

        let _ = EnumDisplayMonitors(None, None, Some(Self::enum_display_monitors_callback), LPARAM(self as *mut WindowManager as isize));
        
        for layout_group in layout_groups {

            for (hmonitor, layouts) in self.layouts.iter_mut() {

                let mut layout = match LayoutGroup::convert_for_monitor(&layout_group, HMONITOR(*hmonitor)) {

                    Some(val) => val,
                    
                    None => layout_group.clone(),
                
                };

                layout.update_all(self.settings.window_padding, self.settings.edge_padding);

                layouts.push(layout);

            }
            
        }

        EnumWindows(Some(Self::enum_windows_callback), LPARAM(self as *mut WindowManager as isize)).unwrap();

        let foreground_window = GetForegroundWindow();

        if self.window_info.contains_key(&foreground_window.0) {

            self.foreground_window = Some(foreground_window);

            self.set_border_to_focused(foreground_window);

        }

        self.update();

    }

    pub unsafe fn add_layout_group(&mut self, layout_group: LayoutGroup) {
        
        for (hmonitor, layouts) in self.layouts.iter_mut() {

            let mut layout = match LayoutGroup::convert_for_monitor(&layout_group, HMONITOR(*hmonitor)) {
                
                Some(val) => val,
            
                None => layout_group.clone(),
            
            };

            layout.update_all(self.settings.window_padding, self.settings.edge_padding);

            layouts.push(layout);

        }

    }

    pub fn get_settings(&self) -> &Settings {

        &self.settings

    }
    
    pub fn get_settings_mut(&mut self) -> &mut Settings {

        &mut self.settings

    }

    pub fn get_monitor_vec(&self) -> &Vec<HMONITOR> {
        
        &self.monitor_handles

    }

    pub unsafe fn window_created(&mut self, hwnd: HWND) {

        if self.ignored_hwnds.contains(&hwnd.0) {

            return;

        }

        let desktop_id;

        let monitor_handle;

        let mut increment_after = None;

        match self.window_info.get_mut(&hwnd.0) {

            Some(info) if info.restored => return,

            Some(info) if is_restored(hwnd) => {

                desktop_id = info.desktop_id;

                monitor_handle = info.monitor_handle;

                increment_after = Some(info.idx);

                match self.workspaces.get_mut(&(desktop_id, monitor_handle.0)) {

                    Some(workspace) => {

                        workspace.managed_window_handles.insert(info.idx, hwnd);

                    },
                    
                    None => {

                        self.workspaces.insert((desktop_id, monitor_handle.0), Workspace::new(hwnd, self.settings.default_layout_idx, self.layouts.get(&monitor_handle.0).unwrap()[self.settings.default_layout_idx].default_idx()));
                        
                    },
                
                };

                info.restored = true;

            },

            None => {

                let mut count = 0;

                loop {

                    match self.virtual_desktop_manager.GetWindowDesktopId(hwnd) {

                        Ok(guid) if guid != GUID::zeroed() => {
                            
                            desktop_id = guid;

                            break;

                        },

                        _ => {

                            count += 1;

                        },

                    }

                    if count == CREATE_RETRIES {

                        return;

                    }

                }

                monitor_handle = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL);

                if monitor_handle.is_invalid() {

                    return;

                }

                match self.workspaces.get_mut(&(desktop_id, monitor_handle.0)) {

                    Some(workspace) => {

                        if is_restored(hwnd) {

                            workspace.managed_window_handles.push(hwnd);

                            self.window_info.insert(hwnd.0, WindowInfo::new(desktop_id, monitor_handle, true, workspace.managed_window_handles.len() - 1));

                            increment_after = Some(workspace.managed_window_handles.len() - 1);

                        }

                        else {

                            self.window_info.insert(hwnd.0, WindowInfo::new(desktop_id, monitor_handle, false, workspace.managed_window_handles.len()));

                        }

                    },
                    
                    None => {

                        if is_restored(hwnd) {

                            self.workspaces.insert((desktop_id, monitor_handle.0), Workspace::new(hwnd, self.settings.default_layout_idx, self.layouts.get(&monitor_handle.0).unwrap()[self.settings.default_layout_idx].default_idx()));
                        
                            self.window_info.insert(hwnd.0, WindowInfo::new(desktop_id, monitor_handle, true, 0));

                            increment_after = Some(0);
                    
                        }

                        else {

                            self.window_info.insert(hwnd.0, WindowInfo::new(desktop_id, monitor_handle, false, 0));

                        }

                    },

                };

                self.initialize_border(hwnd);

            },

            _ => return,

        }

        if let Some(after) = increment_after {

            for (h, info) in self.window_info.iter_mut() {

                if 
                    info.desktop_id == desktop_id && 
                    info.monitor_handle == monitor_handle &&
                    info.idx >= after &&
                    *h != hwnd.0
                {

                        info.idx += 1;

                }

            }

        }

        self.update_workspace(desktop_id, monitor_handle);

    }

    pub unsafe fn window_destroyed(&mut self, hwnd: HWND) {

        let info = match self.window_info.get(&hwnd.0) {

            Some(val) => val,

            None => {

                self.ignored_hwnds.remove(&hwnd.0);

                return;

            },

        };

        let WindowInfo { desktop_id, monitor_handle, restored, idx } = info.to_owned();

        self.window_info.remove(&hwnd.0);

        if restored {

            self.remove_hwnd(desktop_id, monitor_handle, idx);

        }

        if self.foreground_window == Some(hwnd) {

            self.foreground_window = None;

        }

        if self.grabbed_window == Some(hwnd) {

            self.grabbed_window = None;

        }

        self.update_workspace(desktop_id, monitor_handle);

    }

    pub unsafe fn window_minimized_or_maximized(&mut self, hwnd: HWND) {

        let info = match self.window_info.get_mut(&hwnd.0) {

            Some(val) if val.restored => val,

            _ => return,

        };

        let WindowInfo { desktop_id, monitor_handle, restored: _, idx } = info.to_owned();

        info.restored = false;

        self.remove_hwnd(desktop_id, monitor_handle, idx);

        match self.grabbed_window {
            
            Some(h) if h == hwnd => {

                self.grabbed_window = None;

            },

            _ => (),

        }

        self.update_workspace(desktop_id, monitor_handle);

    }

    pub unsafe fn window_cloaked(&mut self, hwnd: HWND) {

        let info= match self.window_info.get(&hwnd.0) {
            
            Some(val) => val,
        
            None => return,
        
        };

        let WindowInfo { desktop_id: old_desktop_id, monitor_handle, restored, idx: old_idx } = info.to_owned();

        let new_desktop_id = match self.virtual_desktop_manager.GetWindowDesktopId(hwnd) {

            Ok(guid) if guid != old_desktop_id => guid,

            _ => return,

        };

        let new_idx;

        if restored {

            self.remove_hwnd(old_desktop_id, monitor_handle, old_idx);

            match self.workspaces.get_mut(&(new_desktop_id, monitor_handle.0)) {

                Some(workspace) => {

                    workspace.managed_window_handles.push(hwnd);

                    new_idx = workspace.managed_window_handles.len() - 1;

                },

                None => {

                    self.workspaces.insert((new_desktop_id, monitor_handle.0), Workspace::new(hwnd, self.settings.default_layout_idx, self.layouts.get(&monitor_handle.0).unwrap()[self.settings.default_layout_idx].default_idx()));

                    new_idx = 0;

                }

            }
            
            for (h, info) in self.window_info.iter_mut() {

                if 
                    info.desktop_id == new_desktop_id && 
                    info.monitor_handle == monitor_handle &&
                    info.idx >= new_idx &&
                    *h != hwnd.0
                {

                        info.idx += 1;

                }

            }

        }

        else {

            match self.workspaces.get(&(new_desktop_id, monitor_handle.0)) {

                Some(workspace) => {

                    new_idx = workspace.managed_window_handles.len();

                },
                
                None => {

                    new_idx = 0;

                },

            }

        }

        self.window_info.insert(hwnd.0, WindowInfo::new(new_desktop_id, monitor_handle, restored, new_idx));
       
        self.update_workspace(old_desktop_id, monitor_handle);

        self.update_workspace(new_desktop_id, monitor_handle);

    }
    
    pub unsafe fn foreground_window_changed(&mut self, hwnd: HWND) {
    
        if !self.window_info.contains_key(&hwnd.0) {

            return;

        }

        self.set_border_to_focused(hwnd);

        match self.foreground_window {

            Some(previous_foreground_window) if previous_foreground_window == hwnd => return,

            Some(previous_foreground_window) => {

                self.set_border_to_unfocused(previous_foreground_window);

            },

            None => (),

        }
        
        self.foreground_window = Some(hwnd);

        if is_restored(hwnd) {

            let info = self.window_info.get(&hwnd.0).unwrap();

            let WindowInfo { desktop_id, monitor_handle, .. } = info.to_owned();

            for (h, info) in self.window_info.iter_mut() {

                if 
                    info.desktop_id == desktop_id &&
                    info.monitor_handle == monitor_handle &&
                    !info.restored &&
                    !IsIconic(HWND(*h)).as_bool()
                {

                        let _ = ShowWindow(HWND(*h), SW_MINIMIZE);

                }

            }

        }

    }

    pub unsafe fn window_move_finished(&mut self, hwnd: HWND) {

        let info = match self.window_info.get_mut(&hwnd.0) {

            Some(val) => val,

            None => return,

        };

        let WindowInfo { desktop_id, monitor_handle: original_monitor_handle, restored, idx } = info.to_owned();

        let new_monitor_handle = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL);

        if !restored {

            info.monitor_handle = new_monitor_handle;
            
            info.idx = match self.workspaces.get(&(desktop_id, new_monitor_handle.0)) {

                Some(w) => {

                    w.managed_window_handles.len()

                },
            
                None => {

                    0

                },
            
            };

            return;

        }

        let changed_monitors = original_monitor_handle != new_monitor_handle;

        let mut moved_to = RECT::default();

        GetWindowRect(hwnd, &mut moved_to).unwrap();

        let moved_to_area = (moved_to.right - moved_to.left)*(moved_to.bottom - moved_to.top);

        let workspace;

        if changed_monitors {

            workspace = match self.workspaces.get_mut(&(desktop_id, new_monitor_handle.0)) {

                Some(w) => w,

                None => {

                    self.workspaces.get_mut(&(desktop_id, original_monitor_handle.0)).unwrap().managed_window_handles.remove(idx);

                    self.workspaces.insert((desktop_id, new_monitor_handle.0), Workspace::new(hwnd, self.settings.default_layout_idx, self.layouts.get(&new_monitor_handle.0).unwrap()[self.settings.default_layout_idx].default_idx()));

                    info.monitor_handle = new_monitor_handle;

                    info.idx = 0;
                    
                    self.update_workspace(desktop_id, original_monitor_handle);

                    self.update_workspace(desktop_id, new_monitor_handle);

                    return;

                }
                
            }

        }

        else {

            workspace = match self.workspaces.get_mut(&(desktop_id, original_monitor_handle.0)) {
                
                Some(w) => w,
                
                None => return,
            
            };

        }

        let mut max_overlap_at: (usize, i32) = (workspace.managed_window_handles.len(), 0);

        {

        let positions =

            if changed_monitors {

                let layout = &mut self.layouts.get_mut(&new_monitor_handle.0).unwrap()[workspace.layout_idx].get_layouts_mut()[workspace.variant_idx];

                while layout.positions_len() < workspace.managed_window_handles.len() + 1 {
         
                    layout.extend();

                    layout.update(self.settings.window_padding, self.settings.edge_padding);

                }

                layout.get_positions_at(workspace.managed_window_handles.len())

            }
            
            else {

                self.layouts.get(&original_monitor_handle.0).unwrap()[workspace.layout_idx].get_layouts()[workspace.variant_idx].get_positions_at(workspace.managed_window_handles.len() - 1)

            };

        if !changed_monitors {

            let position = &positions[idx];

            if 
                moved_to.left == position.x &&
                moved_to.top == position.y &&
                moved_to.right - moved_to.left == position.cx &&
                moved_to.bottom - moved_to.top == position.cy
            {
                
                return;

            }

        }

        for (i, p) in positions.iter().enumerate() {

            let left = std::cmp::max(moved_to.left, p.x);

            let top = std::cmp::max(moved_to.top, p.y);

            let right = std::cmp::min(moved_to.right, p.x + p.cx);
            
            let bottom = std::cmp::min(moved_to.bottom, p.y + p.cy);

            let area = (right - left)*(bottom - top);

            if area == moved_to_area {

                max_overlap_at = (i, area);

                break;
            
            }

            else if area > max_overlap_at.1 {

                max_overlap_at = (i, area);

            }

        }
        
        }

        if changed_monitors {

            self.move_windows_across_monitors(desktop_id, original_monitor_handle, new_monitor_handle, idx, max_overlap_at.0);

            self.update_workspace(desktop_id, original_monitor_handle);

            self.update_workspace(desktop_id, new_monitor_handle);

        }

        else {

            if idx != max_overlap_at.0 {

                self.swap_windows(desktop_id, original_monitor_handle, idx, max_overlap_at.0);

            }

            self.update_workspace(desktop_id, original_monitor_handle);
        
        }

    }

    pub unsafe fn focus_previous(&self) {

        let foreground_window = match self.foreground_window {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let info = match self.window_info.get(&foreground_window.0) {
            
            Some(val) if val.restored => val,
        
            _ => return,
        
        };

        let WindowInfo { desktop_id, monitor_handle, restored: _, idx } = info.to_owned();

        let workspace = match self.workspaces.get(&(desktop_id, monitor_handle.0)) {
        
            Some(val) if val.managed_window_handles.len() > 1 => val,

            _ => return,
        
        };

        let to = 
            
            if idx == 0 {

                workspace.managed_window_handles.len() - 1

            }

            else {

                idx - 1

            };

        let _ = SetForegroundWindow(workspace.managed_window_handles[to]);

    }

    pub unsafe fn focus_next(&self) {

        let foreground_window = match self.foreground_window {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let info = match self.window_info.get(&foreground_window.0) {
            
            Some(val) if val.restored => val,
        
            _ => return,
        
        };

        let WindowInfo { desktop_id, monitor_handle, restored: _, idx } = info.to_owned();

        let workspace = match self.workspaces.get(&(desktop_id, monitor_handle.0)) {
        
            Some(val) if val.managed_window_handles.len() > 1 => val,

            _ => return,
        
        };

        let to = 

            if idx == workspace.managed_window_handles.len() - 1 {

                0

            }

            else {

                idx + 1

            };

        let _ = SetForegroundWindow(workspace.managed_window_handles[to]);

    }

    pub unsafe fn swap_previous(&mut self) {

        let foreground_window = match self.foreground_window {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let info = match self.window_info.get(&foreground_window.0) {
            
            Some(val) if val.restored => val,
        
            _ => return,
        
        };

        let WindowInfo { desktop_id, monitor_handle, restored: _, idx } = info.to_owned();

        if self.ignored_combinations.contains(&(desktop_id, monitor_handle.0)) {

                return;

        }

        let workspace = match self.workspaces.get(&(desktop_id, monitor_handle.0)) {
        
            Some(val) if val.managed_window_handles.len() > 1 => val,

            _ => return,
        
        };

        let swap_with =

            if idx == 0 {

                workspace.managed_window_handles.len() - 1

            }

            else {

                idx - 1

            };

        self.swap_windows(desktop_id, monitor_handle, idx, swap_with);

        self.update_workspace(desktop_id, monitor_handle);

    }

    pub unsafe fn swap_next(&mut self) {

        let foreground_window = match self.foreground_window {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let info = match self.window_info.get(&foreground_window.0) {
            
            Some(val) if val.restored => val,
        
            _ => return,
        
        };

        let WindowInfo { desktop_id, monitor_handle, restored: _, idx } = info.to_owned();

        let workspace = match self.workspaces.get(&(desktop_id, monitor_handle.0)) {
        
            Some(val) if val.managed_window_handles.len() > 1 => val,

            _ => return,
        
        };

        if self.ignored_combinations.contains(&(desktop_id, monitor_handle.0)) {

                return;

        }

        let swap_with = 

            if idx == workspace.managed_window_handles.len() - 1 {

                0

            }

            else {

                idx + 1

            };

        self.swap_windows(desktop_id, monitor_handle, idx, swap_with);

        self.update_workspace(desktop_id, monitor_handle);

    }

    pub unsafe fn variant_previous(&mut self) {
        
        let foreground_window = match self.foreground_window {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let info = match self.window_info.get(&foreground_window.0) {
            
            Some(val) if val.restored => val,
        
            _ => return,
        
        };

        let WindowInfo { desktop_id, monitor_handle, .. } = info.to_owned();

        if self.ignored_combinations.contains(&(desktop_id, monitor_handle.0)) {

                return;

        }

        let workspace = match self.workspaces.get_mut(&(desktop_id, monitor_handle.0)) {
        
            Some(val) if val.variant_idx != 0 => val,

            _ => return,
        
        };

        if self.layouts.get(&monitor_handle.0).unwrap()[workspace.layout_idx].layouts_len() == 1 {

            return;

        }

        workspace.variant_idx -= 1;
        
        self.update_workspace(desktop_id, monitor_handle);

    }

    pub unsafe fn variant_next(&mut self) {
        
        let foreground_window = match self.foreground_window {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let info = match self.window_info.get(&foreground_window.0) {
            
            Some(val) if val.restored => val,
        
            _ => return,
        
        };

        let WindowInfo { desktop_id, monitor_handle, .. } = info.to_owned();

        if self.ignored_combinations.contains(&(desktop_id, monitor_handle.0)) {

                return;

        }

        let workspace = match self.workspaces.get_mut(&(desktop_id, monitor_handle.0)) {
        
            Some(val) => val,

            _ => return,
        
        };

        let layouts_len = self.layouts.get(&monitor_handle.0).unwrap()[workspace.layout_idx].layouts_len();

        if 
            layouts_len == 1 ||
            workspace.variant_idx == layouts_len - 1
        {

            return;

        }

        workspace.variant_idx += 1;

        self.update_workspace(desktop_id, monitor_handle);

    }

    pub unsafe fn layout_previous(&mut self) {
        
        let foreground_window = match self.foreground_window {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let info = match self.window_info.get(&foreground_window.0) {
            
            Some(val) if val.restored => val,
        
            _ => return,
        
        };

        let WindowInfo { desktop_id, monitor_handle, .. } = info.to_owned();

        if self.ignored_combinations.contains(&(desktop_id, monitor_handle.0)) {

                return;

        }

        let workspace = match self.workspaces.get_mut(&(desktop_id, monitor_handle.0)) {
        
            Some(val) => val,

            _ => return,
        
        };

        let layouts = self.layouts.get(&monitor_handle.0).unwrap();
        
        if layouts.len() == 1 {

            return;

        }

        if workspace.layout_idx == 0 {

            workspace.layout_idx = layouts.len() - 1;

        }

        else {

            workspace.layout_idx -= 1;
        
        }

        workspace.variant_idx = layouts[workspace.layout_idx].default_idx();
        
        self.update_workspace(desktop_id, monitor_handle);

    }

    pub unsafe fn layout_next(&mut self) {
        
        let foreground_window = match self.foreground_window {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let info = match self.window_info.get(&foreground_window.0) {
            
            Some(val) if val.restored => val,
        
            _ => return,
        
        };

        let WindowInfo { desktop_id, monitor_handle, .. } = info.to_owned();

        if self.ignored_combinations.contains(&(desktop_id, monitor_handle.0)) {

                return;

        }

        let workspace = match self.workspaces.get_mut(&(desktop_id, monitor_handle.0)) {
        
            Some(val) => val,

            _ => return,
        
        };
        
        let layouts = self.layouts.get(&monitor_handle.0).unwrap();

        if layouts.len() == 1 {

            return;

        }

        if workspace.layout_idx == layouts.len() - 1 {

            workspace.layout_idx = 0;

        }

        else {

            workspace.layout_idx += 1;
        
        }
        
        workspace.variant_idx = layouts[workspace.layout_idx].default_idx();
        
        self.update_workspace(desktop_id, monitor_handle);

    }

    pub unsafe fn focus_previous_monitor(&self) {

        if self.monitor_handles.len() <= 1 {

            return;

        }

        let foreground_window = match self.foreground_window {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let info = match self.window_info.get(&foreground_window.0) {
            
            Some(val) if val.restored => val,
        
            _ => return,
        
        };

        let WindowInfo { desktop_id, monitor_handle, .. } = info.to_owned();

        let mut idx = self.monitor_handles.len();

        for i in 0..self.monitor_handles.len() {

            if self.monitor_handles[i] == monitor_handle {

                idx = i;

            }

        }

        if idx == self.monitor_handles.len() {

            return;

        }

        else if idx == 0 {

            idx = self.monitor_handles.len() - 1;

        }

        else {

            idx -= 1;

        }

        let workspace = match self.workspaces.get(&(desktop_id, self.monitor_handles[idx].0)) {
        
            Some(val) if val.managed_window_handles.len() != 0 => val,

            _ => return,
        
        };

        let _ = SetForegroundWindow(workspace.managed_window_handles[0]);

    }

    pub unsafe fn focus_next_monitor(&self) {

        if self.monitor_handles.len() <= 1 {

            return;

        }

        let foreground_window = match self.foreground_window {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let info = match self.window_info.get(&foreground_window.0) {
            
            Some(val) if val.restored => val,
        
            _ => return,
        
        };

        let WindowInfo { desktop_id, monitor_handle, .. } = info.to_owned();

        let mut idx = self.monitor_handles.len();

        for i in 0..self.monitor_handles.len() {

            if self.monitor_handles[i] == monitor_handle {

                idx = i;

            }

        }

        if idx == self.monitor_handles.len() {

            return;

        }

        else if idx == self.monitor_handles.len() - 1 {

            idx = 0;

        }

        else {

            idx += 1;

        }

        let workspace = match self.workspaces.get(&(desktop_id, self.monitor_handles[idx].0)) {
        
            Some(val) if val.managed_window_handles.len() != 0 => val,

            _ => return,
        
        };

        let _ = SetForegroundWindow(workspace.managed_window_handles[0]);

    }

    pub unsafe fn swap_previous_monitor(&mut self) {

        if self.monitor_handles.len() <= 1 {

            return;

        }

        let foreground_window = match self.foreground_window {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let original_dpi = GetDpiForWindow(foreground_window);

        let info = match self.window_info.get(&foreground_window.0) {
            
            Some(val) if val.restored => val,
        
            _ => return,
        
        };

        let WindowInfo { desktop_id, monitor_handle: original_monitor_handle, restored: _, idx: original_window_idx } = info.to_owned();

        if self.ignored_combinations.contains(&(desktop_id, original_monitor_handle.0)) {

                return;

        }

        let mut hmonitor_handlex = self.monitor_handles.len();

        for i in 0..self.monitor_handles.len() {

            if self.monitor_handles[i] == original_monitor_handle {

                hmonitor_handlex = i;

            }

        }

        let mut new_monitor_handle = HMONITOR::default();

        if hmonitor_handlex == self.monitor_handles.len() {

            return;

        }

        else {

            for i in 0..self.monitor_handles.len() {

                if i == self.monitor_handles.len() - 1 {

                    return;

                }

                if hmonitor_handlex == 0 {

                    hmonitor_handlex = self.monitor_handles.len() - 1;

                }

                else {

                    hmonitor_handlex -= 1;

                }

                new_monitor_handle = self.monitor_handles[hmonitor_handlex];

                if !self.ignored_combinations.contains(&(desktop_id, new_monitor_handle.0)) {

                    break;

                }

            }

        }

        match self.workspaces.get(&(desktop_id, new_monitor_handle.0)) {
        
            Some(w) => {

                self.move_windows_across_monitors(desktop_id, original_monitor_handle, new_monitor_handle, original_window_idx, w.managed_window_handles.len());

            },

            None => {
                
                self.remove_hwnd(desktop_id, original_monitor_handle, original_window_idx);

                self.workspaces.insert((desktop_id, new_monitor_handle.0), Workspace::new(foreground_window, self.settings.default_layout_idx, self.layouts.get(&new_monitor_handle.0).unwrap()[self.settings.default_layout_idx].default_idx()));

                let info_mut = self.window_info.get_mut(&foreground_window.0).unwrap();

                info_mut.monitor_handle = new_monitor_handle;

                info_mut.idx = 0;

            },
        
        };

        self.update_workspace(desktop_id, original_monitor_handle);

        self.update_workspace(desktop_id, new_monitor_handle);

        if GetDpiForWindow(foreground_window) != original_dpi {

            let workspace = self.workspaces.get(&(desktop_id, new_monitor_handle.0)).unwrap();

            let layout = &self.layouts.get(&new_monitor_handle.0).unwrap()[workspace.layout_idx].get_layouts()[workspace.variant_idx];

            let position = &layout.get_positions_at(workspace.managed_window_handles.len() - 1)[workspace.managed_window_handles.len() - 1];

            let _ = SetWindowPos(foreground_window, None, position.x, position.y, position.cx, position.cy, SWP_NOZORDER);

        }

    }

    pub unsafe fn swap_next_monitor(&mut self) {

        if self.monitor_handles.len() <= 1 {

            return;

        }

        let foreground_window = match self.foreground_window {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let original_dpi = GetDpiForWindow(foreground_window);

        let info = match self.window_info.get(&foreground_window.0) {
            
            Some(val) if val.restored => val,
        
            _ => return,
        
        };

        let WindowInfo { desktop_id, monitor_handle: original_monitor_handle, restored: _, idx: original_window_idx } = info.to_owned();

        if self.ignored_combinations.contains(&(desktop_id, original_monitor_handle.0)) {

                return;

        }

        let mut hmonitor_handlex = self.monitor_handles.len();

        for i in 0..self.monitor_handles.len() {

            if self.monitor_handles[i] == original_monitor_handle {

                hmonitor_handlex = i;

            }

        }

        let mut new_monitor_handle = HMONITOR::default();

        if hmonitor_handlex == self.monitor_handles.len() {

            return;

        }

        else {

            for i in 0..self.monitor_handles.len() {

                if i == self.monitor_handles.len() - 1 {

                    return;

                }

                if hmonitor_handlex == self.monitor_handles.len() - 1 {

                    hmonitor_handlex = 0;

                }

                else {

                    hmonitor_handlex += 1;

                }

                new_monitor_handle = self.monitor_handles[hmonitor_handlex];

                if !self.ignored_combinations.contains(&(desktop_id, new_monitor_handle.0)) {

                    break;

                }

            }

        }

        match self.workspaces.get(&(desktop_id, new_monitor_handle.0)) {
        
            Some(w) => {

                self.move_windows_across_monitors(desktop_id, original_monitor_handle, new_monitor_handle, original_window_idx, w.managed_window_handles.len());

            },

            None => {
                
                self.remove_hwnd(desktop_id, original_monitor_handle, original_window_idx);

                self.workspaces.insert((desktop_id, new_monitor_handle.0), Workspace::new(foreground_window, self.settings.default_layout_idx, self.layouts.get(&new_monitor_handle.0).unwrap()[self.settings.default_layout_idx].default_idx()));

                let info_mut = self.window_info.get_mut(&foreground_window.0).unwrap();

                info_mut.monitor_handle = new_monitor_handle;

                info_mut.idx = 0;

            },
        
        };

        self.update_workspace(desktop_id, original_monitor_handle);

        self.update_workspace(desktop_id, new_monitor_handle);

        if GetDpiForWindow(foreground_window) != original_dpi {

            let workspace = self.workspaces.get(&(desktop_id, new_monitor_handle.0)).unwrap();

            let layout = &self.layouts.get(&new_monitor_handle.0).unwrap()[workspace.layout_idx].get_layouts()[workspace.variant_idx];

            let position = &layout.get_positions_at(workspace.managed_window_handles.len() - 1)[workspace.managed_window_handles.len() - 1];

            let _ = SetWindowPos(foreground_window, None, position.x, position.y, position.cx, position.cy, SWP_NOZORDER);

        }

    }

    pub fn grab_window(&mut self) {
        
        self.grabbed_window = match self.foreground_window {
            
            Some(hwnd) => {

                match self.window_info.get(&hwnd.0) {

                    Some(val) if val.restored => Some(hwnd),

                    _ => None,

                }

            },

            None => None,

        }

    }
    
    pub unsafe fn release_window(&mut self) {

        let grabbed_window = match self.grabbed_window {
            
            Some(hwnd) => hwnd,

            None => return,

        };
        
        let foreground_window = match self.foreground_window {

            Some(hwnd) if hwnd != self.grabbed_window.unwrap() => hwnd,
            
            _ => return,
        
        };

        let new_info = match self.window_info.get(&foreground_window.0) {

            Some(val) if val.restored => val,

            _ => return

        };

        let WindowInfo { desktop_id: new_desktop_id, monitor_handle: new_monitor_handle, restored: _, idx: new_idx } = new_info.to_owned();

        if self.ignored_combinations.contains(&(new_desktop_id, new_monitor_handle.0)) {

            return;

        }
        
        let WindowInfo { desktop_id: original_desktop_id, monitor_handle: original_monitor_handle, restored: _, idx: original_idx } = self.window_info.get(&self.grabbed_window.unwrap().0).unwrap().to_owned();
        
        if original_desktop_id != new_desktop_id {

            return;

        }
        
        if 
            original_monitor_handle == new_monitor_handle
        {

            self.swap_windows(original_desktop_id, original_monitor_handle, original_idx, new_idx);

            self.update_workspace(original_desktop_id, original_monitor_handle);

        }

        else {

            self.move_windows_across_monitors(original_desktop_id, original_monitor_handle, new_monitor_handle, original_idx, new_idx);

            let original_dpi = GetDpiForWindow(grabbed_window);
            
            self.update_workspace(original_desktop_id, original_monitor_handle);
            
            self.update_workspace(original_desktop_id, new_monitor_handle);

            if GetDpiForWindow(grabbed_window) != original_dpi {

                let workspace = self.workspaces.get(&(original_desktop_id, new_monitor_handle.0)).unwrap();

                let layout = &self.layouts.get(&new_monitor_handle.0).unwrap()[workspace.layout_idx].get_layouts()[workspace.variant_idx];

                let position = &layout.get_positions_at(workspace.managed_window_handles.len() - 1)[new_idx];

                let _ = SetWindowPos(grabbed_window, None, position.x, position.y, position.cx, position.cy, SWP_NOZORDER);

            }

        }

        let _ = SetForegroundWindow(grabbed_window);

        self.grabbed_window = None;

    }

    pub unsafe fn refresh_workspace(&mut self) {

        let foreground_window = match self.foreground_window {

            Some(hwnd) => hwnd,

            None => return,
            
        };

        let WindowInfo { desktop_id, monitor_handle, .. } = match self.window_info.get(&foreground_window.0) {
            
            Some(val) => val,

            None => return,

        };

        let workspace = match self.workspaces.get(&(*desktop_id, monitor_handle.0)) {
            
            Some(val) => val,

            None => return,

        };

        for h in workspace.managed_window_handles.clone() {

            if !IsWindow(Some(h)).as_bool() {

                self.window_destroyed(h);

            }

        }

    }

    pub unsafe fn toggle_workspace(&mut self) {

        let foreground_window = match self.foreground_window {

            Some(hwnd) => hwnd,

            None => return,
            
        };

        let WindowInfo { desktop_id, monitor_handle, .. } = match self.window_info.get(&foreground_window.0) {
            
            Some(val) => val,

            None => return,

        };

        if self.ignored_combinations.contains(&(*desktop_id, monitor_handle.0)) {

            self.ignored_combinations.remove(&(*desktop_id, monitor_handle.0));

            self.update_workspace(*desktop_id, *monitor_handle);

        }

        else {

            self.ignored_combinations.insert((*desktop_id, monitor_handle.0));

        }


    }

    unsafe fn update_workspace(&mut self, guid: GUID, hmonitor: HMONITOR) {

        if self.ignored_combinations.contains(&(guid, hmonitor.0)) {

            return;

        }

        let workspace = match self.workspaces.get(&(guid, hmonitor.0)) {
            
            Some(w) => w,
            
            None => return,
        
        };

        if workspace.managed_window_handles.len() == 0 {

            return;

        }

        let layout = &mut self.layouts.get_mut(&hmonitor.0).unwrap()[workspace.layout_idx].get_layouts_mut()[workspace.variant_idx];

        while layout.positions_len() < workspace.managed_window_handles.len() {
 
            layout.extend();

            layout.update(self.settings.window_padding, self.settings.edge_padding);

        }

        let mut error_indices: Option<Vec<usize>> = None;

        let positions = layout.get_positions_at(workspace.managed_window_handles.len() - 1);

        for (i, hwnd) in workspace.managed_window_handles.iter().enumerate() {

            match SetWindowPos(*hwnd, None, positions[i].x, positions[i].y, positions[i].cx, positions[i].cy, SWP_NOZORDER) {

                Ok(_) => continue,

                Err(_) => {

                    match &mut error_indices {

                        Some(v) => v.push(i),
                        
                        None => {

                            error_indices = Some(vec![i]);

                        },

                    }

                    self.window_info.remove(&hwnd.0);

                    if GetLastError().0 == 5 {
                    
                        self.ignored_hwnds.insert(hwnd.0);

                    }

                },

            }

        }

        if let Some(v) = error_indices {

            for (i, error_idx) in v.iter().enumerate() {

                self.remove_hwnd(guid, hmonitor, *error_idx - i);

            }

            self.update_workspace(guid, hmonitor);

        }

    }

    unsafe fn update(&mut self) {

        let keys: Vec<(GUID, *mut core::ffi::c_void)> = self.workspaces.keys().map(|k| (k.0, k.1)).collect();
        
        for k in keys.iter() {
            
            self.update_workspace(k.0, HMONITOR(k.1));
        }

    }

    fn swap_windows(&mut self, guid: GUID, hmonitor: HMONITOR, i: usize, j: usize) {

        if i == j {
            
            return;
        
        }

        let first_idx = std::cmp::min(i, j);

        let second_idx = std::cmp::max(i, j);

        let managed_window_handles = &mut self.workspaces.get_mut(&(guid, hmonitor.0)).unwrap().managed_window_handles;

        self.window_info.get_mut(&managed_window_handles[first_idx].0).unwrap().idx = second_idx;

        self.window_info.get_mut(&managed_window_handles[second_idx].0).unwrap().idx = first_idx;

        let (first_slice, second_slice) = managed_window_handles.split_at_mut(second_idx);
        
        std::mem::swap(&mut first_slice[first_idx], &mut second_slice[0]);

    }

    fn move_windows_across_monitors(&mut self, guid: GUID, first_hmonitor: HMONITOR, second_hmonitor: HMONITOR, first_idx: usize, second_idx: usize) {

        let hwnd = self.workspaces.get_mut(&(guid, first_hmonitor.0)).unwrap().managed_window_handles.remove(first_idx);

        if self.window_info.get(&hwnd.0).unwrap().restored {

            for info in self.window_info.values_mut() {
                
                if
                    info.desktop_id == guid &&
                    info.monitor_handle == first_hmonitor &&
                    info.idx > first_idx {

                        info.idx -= 1;

                }

            }

        }

        let info = self.window_info.get_mut(&hwnd.0).unwrap();

        self.workspaces.get_mut(&(guid, second_hmonitor.0)).unwrap().managed_window_handles.push(hwnd);

        let last_idx = self.workspaces.get(&(guid, second_hmonitor.0)).unwrap().managed_window_handles.len() - 1;

        info.monitor_handle = second_hmonitor;

        info.idx = last_idx;
        
        self.swap_windows(guid, second_hmonitor, second_idx, last_idx);



    }

    unsafe fn set_border_to_unfocused(&self, hwnd: HWND) {

        let _ = DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR, &self.settings.get_unfocused_border_colour() as *const COLORREF as *const core::ffi::c_void, std::mem::size_of_val(&self.settings.get_unfocused_border_colour()) as u32);

    }

    unsafe fn set_border_to_focused(&self, hwnd: HWND) {

        let _ = DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR, &self.settings.focused_border_colour as *const COLORREF as *const core::ffi::c_void, std::mem::size_of_val(&self.settings.focused_border_colour) as u32);

    }

    unsafe fn initialize_border(&self, hwnd: HWND) {
    
        let corner_preference = 

            if self.settings.disable_rounding {

                DWMWCP_DONOTROUND

            }

            else {

                DWMWCP_DEFAULT

            };

        let _ = DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &corner_preference as *const DWM_WINDOW_CORNER_PREFERENCE as *const core::ffi::c_void, std::mem::size_of_val(&corner_preference) as u32);

        self.set_border_to_unfocused(hwnd);

    }

    fn remove_hwnd(&mut self, guid: GUID, hmonitor: HMONITOR, idx: usize) {

        let workspace = match self.workspaces.get_mut(&(guid, hmonitor.0)) {

            Some(w) if w.managed_window_handles.len() > idx => w,
            
            _ => return,
        
        };

        workspace.managed_window_handles.remove(idx);

        for info in self.window_info.values_mut() {

            if 
                info.desktop_id == guid &&
                info.monitor_handle == hmonitor &&
                info.idx > idx
            {

                info.idx -= 1;

            }

        }

    }

    unsafe extern "system" fn event_handler(_hwineventhook: HWINEVENTHOOK, event: u32, hwnd: HWND, idobject: i32, _idchild: i32, _ideventthread: u32, _dwmseventtime: u32) {

        if !has_sizebox(hwnd) {

            return;

        }

        match event {

            EVENT_OBJECT_SHOW if idobject == OBJID_WINDOW.0 => {

                PostMessageA(None, messages::WINDOW_CREATED, WPARAM(hwnd.0 as usize), LPARAM::default()).unwrap();

            },

            EVENT_OBJECT_DESTROY if idobject == OBJID_WINDOW.0 => {

                PostMessageA(None, messages::WINDOW_DESTROYED, WPARAM(hwnd.0 as usize), LPARAM::default()).unwrap();

            },

            EVENT_OBJECT_LOCATIONCHANGE => {

                if is_restored(hwnd) {

                    PostMessageA(None, messages::WINDOW_RESTORED, WPARAM(hwnd.0 as usize), LPARAM::default()).unwrap();

                }

                else {

                    PostMessageA(None, messages::WINDOW_MINIMIZED_OR_MAXIMIZED, WPARAM(hwnd.0 as usize), LPARAM::default()).unwrap();

                }

            },
            
            EVENT_OBJECT_HIDE if idobject == OBJID_WINDOW.0 => {

                PostMessageA(None, messages::WINDOW_MINIMIZED_OR_MAXIMIZED, WPARAM(hwnd.0 as usize), LPARAM::default()).unwrap();

            },

            EVENT_OBJECT_CLOAKED if idobject == OBJID_WINDOW.0 => {

                PostMessageA(None, messages::WINDOW_CLOAKED, WPARAM(hwnd.0 as usize), LPARAM::default()).unwrap();

            },
        
            EVENT_SYSTEM_FOREGROUND | EVENT_OBJECT_FOCUS => {

                PostMessageA(None, messages::FOREGROUND_WINDOW_CHANGED, WPARAM(hwnd.0 as usize), LPARAM::default()).unwrap();

            },

            EVENT_SYSTEM_MOVESIZEEND => {

                PostMessageA(None, messages::WINDOW_MOVE_FINISHED, WPARAM(hwnd.0 as usize), LPARAM::default()).unwrap();

            },

            _ => return,

        }

    }

    unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {

        let wm = &mut *(lparam.0 as *mut WindowManager);
        
        let desktop_id = match wm.virtual_desktop_manager.GetWindowDesktopId(hwnd) {

            Ok(guid) if guid != GUID::zeroed() => guid,
            
            _ => return true.into(),

        };

        let monitor_handle = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL);

        if monitor_handle.is_invalid() {

            return true.into();

        }

        if 
            !IsWindowVisible(hwnd).as_bool() ||
            !has_sizebox(hwnd)
        {
            
            return true.into();

        }

        match wm.workspaces.get_mut(&(desktop_id, monitor_handle.0)) {

            Some(workspace) => {
                
                if is_restored(hwnd) {

                    workspace.managed_window_handles.push(hwnd);

                    for info in wm.window_info.values_mut() {

                        if 
                            info.desktop_id == desktop_id && 
                            info.monitor_handle == monitor_handle &&
                            !info.restored
                        {

                                info.idx += 1;

                        }

                    }

                    wm.window_info.insert(hwnd.0, WindowInfo::new(desktop_id, monitor_handle, true, workspace.managed_window_handles.len() - 1));

                }

                else {

                    wm.window_info.insert(hwnd.0, WindowInfo::new(desktop_id, monitor_handle, false, workspace.managed_window_handles.len()));

                }

            },
            
            None => {

                if is_restored(hwnd) {

                    wm.workspaces.insert((desktop_id, monitor_handle.0), Workspace::new(hwnd, wm.settings.default_layout_idx, wm.layouts.get(&monitor_handle.0).unwrap()[wm.settings.default_layout_idx].default_idx()));

                    for info in wm.window_info.values_mut() {

                        if 
                            info.desktop_id == desktop_id &&
                            info.monitor_handle == monitor_handle
                        {

                                info.idx = 1;

                        }

                    }

                    wm.window_info.insert(hwnd.0, WindowInfo::new(desktop_id, monitor_handle, true, 0));

                }

                else {

                    wm.window_info.insert(hwnd.0, WindowInfo::new(desktop_id, monitor_handle, false, 0));

                }

            },
        
        }

        wm.initialize_border(hwnd);

        return true.into();

    }

    unsafe extern "system" fn enum_display_monitors_callback(hmonitor: HMONITOR, _hdc: HDC, _hdc_monitor: *mut RECT, dw_data: LPARAM) -> BOOL {

        let wm = &mut *(dw_data.0 as *mut WindowManager);

        wm.monitor_handles.push(hmonitor);
        
        wm.layouts.insert(hmonitor.0, Vec::new());

        return true.into();

    }

}

unsafe fn is_restored(hwnd: HWND) -> bool {
    
    return

        !IsIconic(hwnd).as_bool() &&
        !IsZoomed(hwnd).as_bool() &&
        !IsWindowArranged(hwnd).as_bool() &&
        IsWindowVisible(hwnd).as_bool()

        ;

}

unsafe fn has_sizebox(hwnd: HWND) -> bool {

    GetWindowLongPtrA(hwnd, GWL_STYLE) & WS_SIZEBOX.0 as isize != 0

}

pub unsafe fn register_hotkeys() {
    
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

pub unsafe fn handle_message(msg: MSG, wm: &mut WindowManager) {

    match msg.message {

        messages::WINDOW_CREATED => {

            wm.window_created(HWND(msg.wParam.0 as *mut core::ffi::c_void));

        },

        messages::WINDOW_RESTORED if wm.window_info.contains_key(&(msg.wParam.0 as *mut core::ffi::c_void)) => {

            wm.window_created(HWND(msg.wParam.0 as *mut core::ffi::c_void));

        },

        messages::WINDOW_DESTROYED => {

            wm.window_destroyed(HWND(msg.wParam.0 as *mut core::ffi::c_void));

        },

        messages::WINDOW_MINIMIZED_OR_MAXIMIZED => {

            wm.window_minimized_or_maximized(HWND(msg.wParam.0 as *mut core::ffi::c_void));

        },

        messages::WINDOW_CLOAKED => {

            wm.window_cloaked(HWND(msg.wParam.0 as *mut core::ffi::c_void));

        },

        messages::FOREGROUND_WINDOW_CHANGED => {

            wm.foreground_window_changed(HWND(msg.wParam.0 as *mut core::ffi::c_void));

        },

        messages::WINDOW_MOVE_FINISHED => {

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

pub unsafe fn show_error_message(message: &str) {

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
