use himewm_layout::*;

use serde::{

    Deserialize,

    Serialize

};

use windows::{

    core::*,

    Win32::{

        Foundation::*, 
        
        Graphics::{
        
            Dwm::*, Gdi::*
        
        }, 
        
        System::Com::*, 
        
        UI::{
    
            Accessibility::*, 
            
            HiDpi::*, 
            
            Shell::*, 
            
            WindowsAndMessaging::*
        
        }

    }

};

pub mod messages {

    use windows::Win32::UI::WindowsAndMessaging::WM_APP;
    
    pub const WINDOW_CREATED: u32 = WM_APP + 1;

    pub const WINDOW_DESTROYED: u32 = WM_APP + 2;
    
    pub const WINDOW_MINIMIZED_OR_MAXIMIZED: u32 = WM_APP + 3;
    
    pub const WINDOW_CLOAKED: u32 = WM_APP + 4;
    
    pub const FOREGROUND_WINDOW_CHANGED: u32 = WM_APP + 5;

    pub const WINDOW_MOVE_FINISHED: u32 = WM_APP + 6;

}

#[derive(Clone)]
pub struct Workspace {
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

#[derive(Debug, Deserialize, Serialize)]
struct Colour {
    r: u8,
    g: u8,
    b: u8,
}

impl Colour {
    
    fn as_colorref(&self) -> COLORREF {

        COLORREF (
            self.r as u32 |
            (self.g as u32) << 8 |
            (self.b as u32) << 16
        )

    }

}

#[derive(Debug, Deserialize, Serialize)]
pub struct Settings {
    default_layout_idx: usize,
    window_padding: i32,
    edge_padding: i32,
    disable_rounding: bool,
    disable_unfocused_border: bool,
    focused_border_colour: Colour
}

impl Default for Settings {

    fn default() -> Self {
    
        Settings {
            default_layout_idx: 0,
            window_padding: 0,
            edge_padding: 0,
            disable_rounding: false,
            disable_unfocused_border: false,
            focused_border_colour: Colour { r: 255, g: 255, b: 255 },
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

pub struct WindowManager {
    virtual_desktop_manager: IVirtualDesktopManager,
    event_hook: HWINEVENTHOOK,
    hmonitors: Vec<HMONITOR>,
    hwnd_locations: std::collections::HashMap<*mut core::ffi::c_void, (GUID, HMONITOR, bool, usize)>, 
    workspaces: std::collections::HashMap<(GUID, *mut core::ffi::c_void), Workspace>,
    foreground_hwnd: Option<HWND>,
    layouts: std::collections::HashMap<*mut core::ffi::c_void, Vec<LayoutGroup>>,
    settings: Settings,
    grabbed_window: Option<HWND>,
    ignored_combinations: std::collections::HashSet<(GUID, *mut core::ffi::c_void)>,
    ignored_hwnds: std::collections::HashSet<*mut core::ffi::c_void>,
}

impl WindowManager {

    pub unsafe fn new(settings: Settings) -> Self {

        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        WindowManager {
            virtual_desktop_manager: CoCreateInstance(&VirtualDesktopManager, None, CLSCTX_INPROC_SERVER).unwrap(),
            event_hook: SetWinEventHook(EVENT_MIN, EVENT_MAX, None, Some(Self::event_handler), 0, 0, WINEVENT_OUTOFCONTEXT),
            hmonitors: Vec::new(),
            hwnd_locations: std::collections::HashMap::new(),
            workspaces: std::collections::HashMap::new(),
            foreground_hwnd: None,
            layouts: std::collections::HashMap::new(),
            settings,
            grabbed_window: None,
            ignored_combinations: std::collections::HashSet::new(),
            ignored_hwnds: std::collections::HashSet::new(),
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

        let foreground_hwnd = GetForegroundWindow();

        if self.hwnd_locations.contains_key(&foreground_hwnd.0) {

            self.foreground_hwnd = Some(foreground_hwnd);

            self.set_border_to_focused(foreground_hwnd);

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
        
        &self.hmonitors

    }

    pub unsafe fn window_created(&mut self, hwnd: HWND) {

        if self.ignored_hwnds.contains(&hwnd.0) {

            return;

        }

        let window_desktop_id;

        let monitor_id;

        let mut increment_after = None;

        match self.hwnd_locations.get(&hwnd.0) {

            Some((_, _, false, _)) => return,

            Some((guid, hmonitor, _, idx)) if is_restored(hwnd) => {

                window_desktop_id = *guid;

                monitor_id = *hmonitor;

                increment_after = Some(*idx);

                match self.workspaces.get_mut(&(window_desktop_id, monitor_id.0)) {

                    Some(workspace) => {

                        workspace.managed_window_handles.insert(*idx, hwnd);

                    },
                    
                    None => {

                        self.workspaces.insert((window_desktop_id, monitor_id.0), Workspace::new(hwnd, self.settings.default_layout_idx, self.layouts.get(&monitor_id.0).unwrap()[self.settings.default_layout_idx].default_idx()));
                        
                    },
                
                };

                self.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, false, *idx));

            },

            None => {

                let start_instant = std::time::Instant::now();

                'timeout: loop {
                    
                    while std::time::Instant::now() - start_instant < std::time::Duration::from_secs(1) {
                        
                        match self.virtual_desktop_manager.GetWindowDesktopId(hwnd) {

                            Ok(guid) if guid != GUID::zeroed() => {

                                window_desktop_id = guid;

                                break 'timeout;

                            },

                            _ => continue,
                        }
                        
                    }

                    return;

                }

                monitor_id = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL);

                if monitor_id.is_invalid() {

                    return;

                }

                match self.workspaces.get_mut(&(window_desktop_id, monitor_id.0)) {

                    Some(workspace) => {

                        if is_restored(hwnd) {

                            workspace.managed_window_handles.push(hwnd);

                            self.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, false, workspace.managed_window_handles.len() - 1));

                            increment_after = Some(workspace.managed_window_handles.len() - 1);

                        }

                        else {

                            self.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, true, workspace.managed_window_handles.len()));

                        }

                    },
                    
                    None => {

                        if is_restored(hwnd) {

                            self.workspaces.insert((window_desktop_id, monitor_id.0), Workspace::new(hwnd, self.settings.default_layout_idx, self.layouts.get(&monitor_id.0).unwrap()[self.settings.default_layout_idx].default_idx()));
                        
                            self.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, false, 0));

                            increment_after = Some(0);
                    
                        }

                        else {

                            self.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, true, 0));

                        }

                    },

                };

                self.initialize_border(hwnd);

            },

            _ => return,

        }

        if let Some(after) = increment_after {

            for (h, (guid, hmonitor, _, i)) in self.hwnd_locations.iter_mut() {

                if 
                    *guid == window_desktop_id && 
                    *hmonitor == monitor_id &&
                    *i >= after &&
                    *h != hwnd.0
                {

                        *i += 1;

                }

            }

        }

        self.update_workspace(window_desktop_id, monitor_id);

    }

    pub unsafe fn window_destroyed(&mut self, hwnd: HWND) {

        let location = match self.hwnd_locations.get(&hwnd.0) {

            Some(val) => val,

            None => {

                self.ignored_hwnds.remove(&hwnd.0);

                return;

            },

        };

        let (window_desktop_id, monitor_id, flag, idx) = location.to_owned();

        self.hwnd_locations.remove(&hwnd.0);

        if !flag {

            self.remove_hwnd(window_desktop_id, monitor_id, idx);

        }

        if self.foreground_hwnd == Some(hwnd) {

            self.foreground_hwnd = None;

        }

        if self.grabbed_window == Some(hwnd) {

            self.grabbed_window = None;

        }

        self.update_workspace(window_desktop_id, monitor_id);

    }

    pub unsafe fn window_minimized_or_maximized(&mut self, hwnd: HWND) {

        let location = match self.hwnd_locations.get_mut(&hwnd.0) {

            Some((_, _, true, _)) | None => return,

            Some(val) => val,

        };

        let (window_desktop_id, monitor_id, _, idx) = location.to_owned();

        location.2 = true;

        self.remove_hwnd(window_desktop_id, monitor_id, idx);

        match self.grabbed_window {
            
            Some(h) if h == hwnd => {

                self.grabbed_window = None;

            },

            _ => (),

        }

        self.update_workspace(window_desktop_id, monitor_id);

    }

    pub unsafe fn window_cloaked(&mut self, hwnd: HWND) {

        let location= match self.hwnd_locations.get(&hwnd.0) {
            
            Some(val) => val,
        
            None => return,
        
        };

        let (old_window_desktop_id, monitor_id, flag, old_idx) = location.to_owned();

        let new_window_desktop_id = match self.virtual_desktop_manager.GetWindowDesktopId(hwnd) {

            Ok(guid) if guid != old_window_desktop_id => guid,

            _ => return,

        };

        let new_idx;

        if !flag {

            self.remove_hwnd(old_window_desktop_id, monitor_id, old_idx);

            match self.workspaces.get_mut(&(new_window_desktop_id, monitor_id.0)) {

                Some(workspace) => {

                    workspace.managed_window_handles.push(hwnd);

                    new_idx = workspace.managed_window_handles.len() - 1;

                },

                None => {

                    self.workspaces.insert((new_window_desktop_id, monitor_id.0), Workspace::new(hwnd, self.settings.default_layout_idx, self.layouts.get(&monitor_id.0).unwrap()[self.settings.default_layout_idx].default_idx()));

                    new_idx = 0;

                }

            }
            
            for (h, (guid, hmonitor, _, i)) in self.hwnd_locations.iter_mut() {

                if 
                    *guid == new_window_desktop_id && 
                    *hmonitor == monitor_id &&
                    *i >= new_idx &&
                    *h != hwnd.0
                {

                        *i += 1;

                }

            }

        }

        else {

            match self.workspaces.get(&(new_window_desktop_id, monitor_id.0)) {

                Some(workspace) => {

                    new_idx = workspace.managed_window_handles.len();

                },
                
                None => {

                    new_idx = 0;

                },

            }

        }

        self.hwnd_locations.insert(hwnd.0, (new_window_desktop_id, monitor_id, flag, new_idx));
       
        self.update_workspace(old_window_desktop_id, monitor_id);

        self.update_workspace(new_window_desktop_id, monitor_id);

    }
    
    pub unsafe fn foreground_window_changed(&mut self, hwnd: HWND) {
    
        if !self.hwnd_locations.contains_key(&hwnd.0) {

            return;

        }

        self.set_border_to_focused(hwnd);

        match self.foreground_hwnd {

            Some(previous_foreground_hwnd) if previous_foreground_hwnd == hwnd => return,

            Some(previous_foreground_hwnd) => {

                self.set_border_to_unfocused(previous_foreground_hwnd);

            },

            None => (),

        }
        
        self.foreground_hwnd = Some(hwnd);

        if is_restored(hwnd) {

            let location = self.hwnd_locations.get(&hwnd.0).unwrap();

            let (window_desktop_id, monitor_id, _, _) = location.to_owned();

            for (h, (guid, hmonitor, flag, _)) in self.hwnd_locations.iter_mut() {

                if 
                    *guid == window_desktop_id &&
                    *hmonitor == monitor_id &&
                    *flag &&
                    !IsIconic(HWND(*h)).as_bool()
                {

                        let _ = ShowWindow(HWND(*h), SW_MINIMIZE);

                }

            }

        }

    }

    pub unsafe fn window_move_finished(&mut self, hwnd: HWND) {

        let location = match self.hwnd_locations.get_mut(&hwnd.0) {

            Some(val) => val,

            None => return,

        };

        let (window_desktop_id, original_monitor_id, flag, idx) = location.to_owned();

        let new_monitor_id = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL);

        if flag {

            location.1 = new_monitor_id;
            
            location.3 = match self.workspaces.get(&(window_desktop_id, new_monitor_id.0)) {

                Some(w) => {

                    w.managed_window_handles.len()

                },
            
                None => {

                    0

                },
            
            };

            return;

        }

        let changed_monitors = original_monitor_id != new_monitor_id;

        let mut moved_to = RECT::default();

        GetWindowRect(hwnd, &mut moved_to).unwrap();

        let moved_to_area = (moved_to.right - moved_to.left)*(moved_to.bottom - moved_to.top);

        let workspace;

        if changed_monitors {

            workspace = match self.workspaces.get_mut(&(window_desktop_id, new_monitor_id.0)) {

                Some(w) => w,

                None => {

                    self.workspaces.get_mut(&(window_desktop_id, original_monitor_id.0)).unwrap().managed_window_handles.remove(idx);

                    self.workspaces.insert((window_desktop_id, new_monitor_id.0), Workspace::new(hwnd, self.settings.default_layout_idx, self.layouts.get(&new_monitor_id.0).unwrap()[self.settings.default_layout_idx].default_idx()));

                    location.1 = new_monitor_id;

                    location.3 = 0;
                    
                    self.update_workspace(window_desktop_id, original_monitor_id);

                    self.update_workspace(window_desktop_id, new_monitor_id);

                    return;

                }
                
            }

        }

        else {

            workspace = match self.workspaces.get_mut(&(window_desktop_id, original_monitor_id.0)) {
                
                Some(w) => w,
                
                None => return,
            
            };

        }

        let mut max_overlap_at: (usize, i32) = (workspace.managed_window_handles.len(), 0);

        {

        let positions =

            if changed_monitors {

                let layout = &mut self.layouts.get_mut(&new_monitor_id.0).unwrap()[workspace.layout_idx].get_layouts_mut()[workspace.variant_idx];

                while layout.positions_len() < workspace.managed_window_handles.len() + 1 {
         
                    layout.extend();

                    layout.update(self.settings.window_padding, self.settings.edge_padding);

                }

                layout.get(workspace.managed_window_handles.len())

            }
            
            else {

                self.layouts.get(&original_monitor_id.0).unwrap()[workspace.layout_idx].get_layouts()[workspace.variant_idx].get(workspace.managed_window_handles.len() - 1)

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

            self.move_windows_across_monitors(window_desktop_id, original_monitor_id, new_monitor_id, idx, max_overlap_at.0);

            self.update_workspace(window_desktop_id, original_monitor_id);

            self.update_workspace(window_desktop_id, new_monitor_id);

        }

        else {

            if idx != max_overlap_at.0 {

                self.swap_windows(window_desktop_id, original_monitor_id, idx, max_overlap_at.0);

            }

            self.update_workspace(window_desktop_id, original_monitor_id);
        
        }

    }

    pub unsafe fn focus_previous(&self) {

        let foreground_hwnd = match self.foreground_hwnd {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let location = match self.hwnd_locations.get(&foreground_hwnd.0) {
            
            Some(val) if !val.2 => val,
        
            _ => return,
        
        };

        let (window_desktop_id, monitor_id, _, idx) = location.to_owned();

        let workspace = match self.workspaces.get(&(window_desktop_id, monitor_id.0)) {
        
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

        let foreground_hwnd = match self.foreground_hwnd {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let location = match self.hwnd_locations.get(&foreground_hwnd.0) {

            Some(val) if !val.2 => val,
        
            _ => return,
        
        };

        let (window_desktop_id, monitor_id, _, idx) = location.to_owned();

        let workspace = match self.workspaces.get(&(window_desktop_id, monitor_id.0)) {
        
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

        let foreground_hwnd = match self.foreground_hwnd {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let location = match self.hwnd_locations.get(&foreground_hwnd.0) {
            
            Some(val) if !val.2 => val,
        
            _ => return,
        
        };

        let (window_desktop_id, monitor_id, _, idx) = location.to_owned();

        if self.ignored_combinations.contains(&(window_desktop_id, monitor_id.0)) {

                return;

        }

        let workspace = match self.workspaces.get(&(window_desktop_id, monitor_id.0)) {
        
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

        self.swap_windows(window_desktop_id, monitor_id, idx, swap_with);

        self.update_workspace(window_desktop_id, monitor_id);

    }

    pub unsafe fn swap_next(&mut self) {

        let foreground_hwnd = match self.foreground_hwnd {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let location = match self.hwnd_locations.get(&foreground_hwnd.0) {
            
            Some(val) if !val.2 => val,
        
            _ => return,
        
        };

        let (window_desktop_id, monitor_id, _, idx) = location.to_owned();

        let workspace = match self.workspaces.get(&(window_desktop_id, monitor_id.0)) {
        
            Some(val) if val.managed_window_handles.len() > 1 => val,

            _ => return,
        
        };

        if self.ignored_combinations.contains(&(window_desktop_id, monitor_id.0)) {

                return;

        }

        let swap_with = 

            if idx == workspace.managed_window_handles.len() - 1 {

                0

            }

            else {

                idx + 1

            };

        self.swap_windows(window_desktop_id, monitor_id, idx, swap_with);

        self.update_workspace(window_desktop_id, monitor_id);

    }

    pub unsafe fn variant_previous(&mut self) {
        
        let foreground_hwnd = match self.foreground_hwnd {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let location = match self.hwnd_locations.get(&foreground_hwnd.0) {
            
            Some(val) if !val.2 => val,
        
            _ => return,
        
        };

        let (window_desktop_id, monitor_id, _, _) = location.to_owned();

        if self.ignored_combinations.contains(&(window_desktop_id, monitor_id.0)) {

                return;

        }

        let workspace = match self.workspaces.get_mut(&(window_desktop_id, monitor_id.0)) {
        
            Some(val) if val.variant_idx != 0 => val,

            _ => return,
        
        };

        if self.layouts.get(&monitor_id.0).unwrap()[workspace.layout_idx].layouts_len() == 1 {

            return;

        }

        workspace.variant_idx -= 1;
        
        self.update_workspace(window_desktop_id, monitor_id);

    }

    pub unsafe fn variant_next(&mut self) {
        
        let foreground_hwnd = match self.foreground_hwnd {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let location = match self.hwnd_locations.get(&foreground_hwnd.0) {
            
            Some(val) if !val.2 => val,
        
            _ => return,
        
        };

        let (window_desktop_id, monitor_id, _, _) = location.to_owned();

        if self.ignored_combinations.contains(&(window_desktop_id, monitor_id.0)) {

                return;

        }

        let workspace = match self.workspaces.get_mut(&(window_desktop_id, monitor_id.0)) {
        
            Some(val) => val,

            _ => return,
        
        };

        let layouts_len = self.layouts.get(&monitor_id.0).unwrap()[workspace.layout_idx].layouts_len();

        if 
            layouts_len == 1 ||
            workspace.variant_idx == layouts_len - 1
        {

            return;

        }

        workspace.variant_idx += 1;

        self.update_workspace(window_desktop_id, monitor_id);

    }

    pub unsafe fn layout_previous(&mut self) {
        
        let foreground_hwnd = match self.foreground_hwnd {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let location = match self.hwnd_locations.get(&foreground_hwnd.0) {
            
            Some(val) if !val.2 => val,
        
            _ => return,
        
        };

        let (window_desktop_id, monitor_id, _, _) = location.to_owned();

        if self.ignored_combinations.contains(&(window_desktop_id, monitor_id.0)) {

                return;

        }

        let workspace = match self.workspaces.get_mut(&(window_desktop_id, monitor_id.0)) {
        
            Some(val) => val,

            _ => return,
        
        };

        let layouts = self.layouts.get(&monitor_id.0).unwrap();
        
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
        
        self.update_workspace(window_desktop_id, monitor_id);

    }

    pub unsafe fn layout_next(&mut self) {
        
        let foreground_hwnd = match self.foreground_hwnd {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let location = match self.hwnd_locations.get(&foreground_hwnd.0) {

            Some(val) if !val.2 => val,
        
            _ => return,
        
        };
        
        let (window_desktop_id, monitor_id, _, _) = location.to_owned();

        if self.ignored_combinations.contains(&(window_desktop_id, monitor_id.0)) {

                return;

        }

        let workspace = match self.workspaces.get_mut(&(window_desktop_id, monitor_id.0)) {
        
            Some(val) => val,

            _ => return,
        
        };
        
        let layouts = self.layouts.get(&monitor_id.0).unwrap();

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
        
        self.update_workspace(window_desktop_id, monitor_id);

    }

    pub unsafe fn focus_previous_monitor(&self) {

        if self.hmonitors.len() <= 1 {

            return;

        }

        let foreground_hwnd = match self.foreground_hwnd {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let location = match self.hwnd_locations.get(&foreground_hwnd.0) {
            
            Some(val) if !val.2 => val,
        
            _ => return,
        
        };

        let (window_desktop_id, monitor_id, _, _) = location.to_owned();

        let mut idx = self.hmonitors.len();

        for i in 0..self.hmonitors.len() {

            if self.hmonitors[i] == monitor_id {

                idx = i;

            }

        }

        if idx == self.hmonitors.len() {

            return;

        }

        else if idx == 0 {

            idx = self.hmonitors.len() - 1;

        }

        else {

            idx -= 1;

        }

        let workspace = match self.workspaces.get(&(window_desktop_id, self.hmonitors[idx].0)) {
        
            Some(val) if val.managed_window_handles.len() != 0 => val,

            _ => return,
        
        };

        let _ = SetForegroundWindow(workspace.managed_window_handles[0]);

    }

    pub unsafe fn focus_next_monitor(&self) {

        if self.hmonitors.len() <= 1 {

            return;

        }

        let foreground_hwnd = match self.foreground_hwnd {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let location = match self.hwnd_locations.get(&foreground_hwnd.0) {
            
            Some(val) if !val.2 => val,
        
            _ => return,
        
        };
        
        let (window_desktop_id, monitor_id, _, _) = location.to_owned();

        let mut idx = self.hmonitors.len();

        for i in 0..self.hmonitors.len() {

            if self.hmonitors[i] == monitor_id {

                idx = i;

            }

        }

        if idx == self.hmonitors.len() {

            return;

        }

        else if idx == self.hmonitors.len() - 1 {

            idx = 0;

        }

        else {

            idx += 1;

        }

        let workspace = match self.workspaces.get(&(window_desktop_id, self.hmonitors[idx].0)) {
        
            Some(val) if val.managed_window_handles.len() != 0 => val,

            _ => return,
        
        };

        let _ = SetForegroundWindow(workspace.managed_window_handles[0]);

    }

    pub unsafe fn swap_previous_monitor(&mut self) {

        if self.hmonitors.len() <= 1 {

            return;

        }

        let foreground_hwnd = match self.foreground_hwnd {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let original_dpi = GetDpiForWindow(foreground_hwnd);

        let location = match self.hwnd_locations.get(&foreground_hwnd.0) {
            
            Some(val) if !val.2 => val,
        
            _ => return,
        
        };

        let (window_desktop_id, original_monitor_id, _, original_window_idx) = location.to_owned();

        if self.ignored_combinations.contains(&(window_desktop_id, original_monitor_id.0)) {

                return;

        }

        let mut hmonitor_idx = self.hmonitors.len();

        for i in 0..self.hmonitors.len() {

            if self.hmonitors[i] == original_monitor_id {

                hmonitor_idx = i;

            }

        }

        let mut new_monitor_id = HMONITOR::default();

        if hmonitor_idx == self.hmonitors.len() {

            return;

        }

        else {

            for i in 0..self.hmonitors.len() {

                if i == self.hmonitors.len() - 1 {

                    return;

                }

                if hmonitor_idx == 0 {

                    hmonitor_idx = self.hmonitors.len() - 1;

                }

                else {

                    hmonitor_idx -= 1;

                }

                new_monitor_id = self.hmonitors[hmonitor_idx];

                if !self.ignored_combinations.contains(&(window_desktop_id, new_monitor_id.0)) {

                    break;

                }

            }

        }

        match self.workspaces.get(&(window_desktop_id, new_monitor_id.0)) {
        
            Some(w) => {

                self.move_windows_across_monitors(window_desktop_id, original_monitor_id, new_monitor_id, original_window_idx, w.managed_window_handles.len());

            },

            None => {
                
                self.remove_hwnd(window_desktop_id, original_monitor_id, original_window_idx);

                self.workspaces.insert((window_desktop_id, new_monitor_id.0), Workspace::new(foreground_hwnd, self.settings.default_layout_idx, self.layouts.get(&new_monitor_id.0).unwrap()[self.settings.default_layout_idx].default_idx()));

                let location_mut = self.hwnd_locations.get_mut(&foreground_hwnd.0).unwrap();

                location_mut.1 = new_monitor_id;

                location_mut.3 = 0;

            },
        
        };

        self.update_workspace(window_desktop_id, original_monitor_id);

        self.update_workspace(window_desktop_id, new_monitor_id);

        if GetDpiForWindow(foreground_hwnd) != original_dpi {

            self.update_workspace(window_desktop_id, new_monitor_id);

        }

    }

    pub unsafe fn swap_next_monitor(&mut self) {

        if self.hmonitors.len() <= 1 {

            return;

        }

        let foreground_hwnd = match self.foreground_hwnd {
            
            Some(hwnd) => hwnd,
        
            None => return,
        
        };

        let original_dpi = GetDpiForWindow(foreground_hwnd);

        let location = match self.hwnd_locations.get(&foreground_hwnd.0) {
            
            Some(val) if !val.2 => val,
        
            _ => return,
        
        };
        
        let (window_desktop_id, original_monitor_id, _, original_window_idx) = location.to_owned();

        if self.ignored_combinations.contains(&(window_desktop_id, original_monitor_id.0)) {

                return;

        }

        let mut hmonitor_idx = self.hmonitors.len();

        for i in 0..self.hmonitors.len() {

            if self.hmonitors[i] == original_monitor_id {

                hmonitor_idx = i;

            }

        }

        let mut new_monitor_id = HMONITOR::default();

        if hmonitor_idx == self.hmonitors.len() {

            return;

        }

        else {

            for i in 0..self.hmonitors.len() {

                if i == self.hmonitors.len() - 1 {

                    return;

                }

                if hmonitor_idx == self.hmonitors.len() - 1 {

                    hmonitor_idx = 0;

                }

                else {

                    hmonitor_idx += 1;

                }

                new_monitor_id = self.hmonitors[hmonitor_idx];

                if !self.ignored_combinations.contains(&(window_desktop_id, new_monitor_id.0)) {

                    break;

                }

            }

        }

        match self.workspaces.get(&(window_desktop_id, new_monitor_id.0)) {
        
            Some(w) => {

                self.move_windows_across_monitors(window_desktop_id, original_monitor_id, new_monitor_id, original_window_idx, w.managed_window_handles.len());

            },

            None => {
                
                self.remove_hwnd(window_desktop_id, original_monitor_id, original_window_idx);

                self.workspaces.insert((window_desktop_id, new_monitor_id.0), Workspace::new(foreground_hwnd, self.settings.default_layout_idx, self.layouts.get(&new_monitor_id.0).unwrap()[self.settings.default_layout_idx].default_idx()));

                let location_mut = self.hwnd_locations.get_mut(&foreground_hwnd.0).unwrap();

                location_mut.1 = new_monitor_id;

                location_mut.3 = 0;

            },
        
        };

        self.update_workspace(window_desktop_id, original_monitor_id);

        self.update_workspace(window_desktop_id, new_monitor_id);

        if GetDpiForWindow(foreground_hwnd) != original_dpi {

            self.update_workspace(window_desktop_id, new_monitor_id);

        }

    }

    pub fn grab_window(&mut self) {
        
        self.grabbed_window = match self.foreground_hwnd {
            
            Some(hwnd) => {

                match self.hwnd_locations.get(&hwnd.0) {

                    Some(val) if !val.2 => Some(hwnd),

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
        
        let foreground_hwnd = match self.foreground_hwnd {

            Some(hwnd) if hwnd != self.grabbed_window.unwrap() => hwnd,
            
            _ => return,
        
        };

        let new_location = match self.hwnd_locations.get(&foreground_hwnd.0) {

            Some(val) if !val.2 => val,

            _ => return

        };

        let (new_window_desktop_id, new_monitor_id, _, new_idx) = new_location.clone();

        if self.ignored_combinations.contains(&(new_window_desktop_id, new_monitor_id.0)) {

            return;

        }
        
        let (original_window_desktop_id, original_monitor_id, _, original_idx) = self.hwnd_locations.get(&self.grabbed_window.unwrap().0).unwrap().to_owned();
        
        if original_window_desktop_id != new_window_desktop_id {

            return;

        }
        
        if 
            original_monitor_id == new_monitor_id
        {

            self.swap_windows(original_window_desktop_id, original_monitor_id, original_idx, new_idx);

            self.update_workspace(original_window_desktop_id, original_monitor_id);

        }

        else {

            self.move_windows_across_monitors(original_window_desktop_id, original_monitor_id, new_monitor_id, original_idx, new_idx);

            let original_dpi = GetDpiForWindow(grabbed_window);
            
            self.update_workspace(original_window_desktop_id, original_monitor_id);
            
            self.update_workspace(original_window_desktop_id, new_monitor_id);

            if GetDpiForWindow(grabbed_window) != original_dpi {
            
                self.update_workspace(original_window_desktop_id, new_monitor_id);

            }

        }

        let _ = SetForegroundWindow(grabbed_window);

        self.grabbed_window = None;

    }

    pub unsafe fn refresh_workspace(&mut self) {

        let foreground_hwnd = match self.foreground_hwnd {

            Some(hwnd) => hwnd,

            None => return,
            
        };

        let (window_desktop_id, monitor_id, _, _) = match self.hwnd_locations.get(&foreground_hwnd.0) {
            
            Some(val) => val,

            None => return,

        };

        let workspace = match self.workspaces.get(&(*window_desktop_id, monitor_id.0)) {
            
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

        let foreground_hwnd = match self.foreground_hwnd {

            Some(hwnd) => hwnd,

            None => return,
            
        };

        let (window_desktop_id, monitor_id, _, _) = match self.hwnd_locations.get(&foreground_hwnd.0) {
            
            Some(val) => val,

            None => return,

        };

        if self.ignored_combinations.contains(&(*window_desktop_id, monitor_id.0)) {

            self.ignored_combinations.remove(&(*window_desktop_id, monitor_id.0));

            self.update_workspace(*window_desktop_id, *monitor_id);

        }

        else {

            self.ignored_combinations.insert((*window_desktop_id, monitor_id.0));

        }


    }

    unsafe fn update_workspace(&mut self, guid: GUID, hmonitor: HMONITOR) {

        if self.ignored_combinations.contains(&(guid, hmonitor.0)) {

            return;

        }

        let workspace = match self.workspaces.get_mut(&(guid, hmonitor.0)) {
            
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

        let positions = layout.get(workspace.managed_window_handles.len() - 1);

        for (i, hwnd) in workspace.managed_window_handles.iter().enumerate() {

            match SetWindowPos(*hwnd, None, positions[i].x, positions[i].y, positions[i].cx, positions[i].cy, SWP_NOZORDER) {

                Err(_) if GetLastError().0 == 5 => {

                    match &mut error_indices {

                        Some(v) => v.push(i),
                        
                        None => {

                            error_indices = Some(vec![i]);

                        },

                    }

                    self.hwnd_locations.remove(&hwnd.0);

                    self.ignored_hwnds.insert(hwnd.0);

                },

                _ => continue,

            }

        }
        
        if let Some(v) = error_indices {

            for (i, error_idx) in v.iter().enumerate() {

                workspace.managed_window_handles.remove(*error_idx - i);

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

        self.hwnd_locations.get_mut(&managed_window_handles[first_idx].0).unwrap().3 = second_idx;

        self.hwnd_locations.get_mut(&managed_window_handles[second_idx].0).unwrap().3 = first_idx;

        let (first_slice, second_slice) = managed_window_handles.split_at_mut(second_idx);
        
        std::mem::swap(&mut first_slice[first_idx], &mut second_slice[0]);

    }

    fn move_windows_across_monitors(&mut self, guid: GUID, first_hmonitor: HMONITOR, second_hmonitor: HMONITOR, first_idx: usize, second_idx: usize) {

        let hwnd = self.workspaces.get_mut(&(guid, first_hmonitor.0)).unwrap().managed_window_handles.remove(first_idx);

        if !self.hwnd_locations.get(&hwnd.0).unwrap().2 {

            for (g, hmonitor, _, i) in self.hwnd_locations.values_mut() {
                
                if
                    *g == guid &&
                    *hmonitor == first_hmonitor &&
                    *i > first_idx {

                        *i -= 1;

                }

            }

        }

        let location = self.hwnd_locations.get_mut(&hwnd.0).unwrap();

        self.workspaces.get_mut(&(guid, second_hmonitor.0)).unwrap().managed_window_handles.push(hwnd);

        let last_idx = self.workspaces.get(&(guid, second_hmonitor.0)).unwrap().managed_window_handles.len() - 1;

        location.1 = second_hmonitor;

        location.3 = last_idx;
        
        self.swap_windows(guid, second_hmonitor, second_idx, last_idx);



    }

    unsafe fn set_border_to_unfocused(&self, hwnd: HWND) {

        let _ = DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR, &self.settings.get_unfocused_border_colour() as *const COLORREF as *const core::ffi::c_void, std::mem::size_of_val(&self.settings.get_unfocused_border_colour()) as u32);

    }

    unsafe fn set_border_to_focused(&self, hwnd: HWND) {

        let _ = DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR, &self.settings.focused_border_colour.as_colorref() as *const COLORREF as *const core::ffi::c_void, std::mem::size_of_val(&self.settings.focused_border_colour.as_colorref()) as u32);

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

        for (g, h, _, i) in self.hwnd_locations.values_mut() {

            if 
                *g == guid &&
                *h == hmonitor &&
                *i > idx
            {

                *i -= 1;

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

                    PostMessageA(None, messages::WINDOW_CREATED, WPARAM(hwnd.0 as usize), LPARAM::default()).unwrap();

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
        
        let window_desktop_id = match wm.virtual_desktop_manager.GetWindowDesktopId(hwnd) {

            Ok(guid) if guid != GUID::zeroed() => guid,
            
            _ => return true.into(),

        };

        let monitor_id = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL);

        if monitor_id.is_invalid() {

            return true.into();

        }

        if 
            !IsWindowVisible(hwnd).as_bool() ||
            !has_sizebox(hwnd) ||
            IsIconic(hwnd).as_bool()
        {
            
            return true.into();

        }

        match wm.workspaces.get_mut(&(window_desktop_id, monitor_id.0)) {

            Some(workspace) => {
                
                if is_restored(hwnd) {

                    workspace.managed_window_handles.push(hwnd);

                    for (guid, hmonitor, flag, i) in wm.hwnd_locations.values_mut() {

                        if 
                            *guid == window_desktop_id && 
                            *hmonitor == monitor_id &&
                            *flag
                        {

                                *i += 1;

                        }

                    }

                    wm.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, false, workspace.managed_window_handles.len() - 1));

                }

                else {

                    wm.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, true, workspace.managed_window_handles.len()));

                }

            },
            
            None => {

                if is_restored(hwnd) {

                    wm.workspaces.insert((window_desktop_id, monitor_id.0), Workspace::new(hwnd, wm.settings.default_layout_idx, wm.layouts.get(&monitor_id.0).unwrap()[wm.settings.default_layout_idx].default_idx()));

                    for (guid, hmonitor, _, i) in wm.hwnd_locations.values_mut() {

                        if 
                            *guid == window_desktop_id &&
                            *hmonitor == monitor_id
                        {

                                *i = 1;

                        }

                    }

                    wm.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, false, 0));

                }

                else {

                    wm.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, true, 0));

                }

            },
        
        }

        wm.initialize_border(hwnd);

        return true.into();

    }

    unsafe extern "system" fn enum_display_monitors_callback(hmonitor: HMONITOR, _hdc: HDC, _hdc_monitor: *mut RECT, dw_data: LPARAM) -> BOOL {

        let wm = &mut *(dw_data.0 as *mut WindowManager);

        wm.hmonitors.push(hmonitor);
        
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
