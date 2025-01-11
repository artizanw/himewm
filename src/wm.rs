pub mod messages {
    
    pub const WINDOW_CREATED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 1;

    pub const WINDOW_DESTROYED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 2;
    
    pub const WINDOW_MINIMIZED_OR_MAXIMIZED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 3;
    
    pub const WINDOW_CLOAKED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 4;
    
    pub const FOREGROUND_WINDOW_CHANGED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 5;

    pub const WINDOW_MOVE_FINISHED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 6;

}

#[derive(Clone)]
pub struct Workspace {
    layout: std::sync::Arc<std::sync::RwLock<crate::layout::Layout>>,
    pub managed_window_handles: Vec<windows::Win32::Foundation::HWND>,
}

impl Workspace {

    unsafe fn new(hwnd: windows::Win32::Foundation::HWND, layout: std::sync::Arc<std::sync::RwLock<crate::layout::Layout>>) -> Self {

        Workspace {
            layout,
            managed_window_handles: vec![hwnd],
        }

    }

}

#[derive(Debug)]
pub struct WindowSettings {
    disable_rounding: bool,
    disable_unfocused_border: bool,
    focused_border_colour: windows::Win32::Foundation::COLORREF
}

impl Default for WindowSettings {

    fn default() -> Self {
    
        WindowSettings { 
            disable_rounding: false,
            disable_unfocused_border: false,
            focused_border_colour: windows::Win32::Foundation::COLORREF(0x00FFFFFF),
        }
    
    }
}

impl WindowSettings {

    pub fn set_disable_rounding(&mut self, val: bool) {

        self.disable_rounding = val;

    }

    pub fn set_disable_unfocused_border(&mut self, val: bool) {
        
        self.disable_unfocused_border = val;

    }

    pub fn set_focused_border_colour(&mut self, val: windows::Win32::Foundation::COLORREF) {
        
        self.focused_border_colour = val;

    }

    fn get_unfocused_border_colour(&self) -> windows::Win32::Foundation::COLORREF {

        if self.disable_unfocused_border {

            return windows::Win32::Foundation::COLORREF(windows::Win32::Graphics::Dwm::DWMWA_COLOR_NONE);

        }

        else {

            return windows::Win32::Foundation::COLORREF(windows::Win32::Graphics::Dwm::DWMWA_COLOR_DEFAULT);

        }

    }

}

pub struct WindowManager {
    virtual_desktop_manager: windows::Win32::UI::Shell::IVirtualDesktopManager,
    event_hook: windows::Win32::UI::Accessibility::HWINEVENTHOOK,
    pub hmonitor_default_layout_indices: std::collections::HashMap<*mut core::ffi::c_void, usize>,
    hwnd_locations: std::collections::HashMap<*mut core::ffi::c_void, (windows::core::GUID, windows::Win32::Graphics::Gdi::HMONITOR, bool, usize)>, 
    pub workspaces: std::collections::HashMap<(windows::core::GUID, *mut core::ffi::c_void), Workspace>,
    foreground_hwnd: Option<windows::Win32::Foundation::HWND>,
    layouts: std::collections::HashMap<*mut core::ffi::c_void, Vec<std::sync::Arc<std::sync::RwLock<crate::layout::Layout>>>>,
    window_settings: WindowSettings,
    ignored_combinations: std::collections::HashSet<(windows::core::GUID, *mut core::ffi::c_void)>,
    ignored_hwnds: std::collections::HashSet<*mut core::ffi::c_void>,
}

impl WindowManager {

    pub unsafe fn new() -> Self {

        windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);

        WindowManager {
            virtual_desktop_manager: windows::Win32::System::Com::CoCreateInstance(&windows::Win32::UI::Shell::VirtualDesktopManager, None, windows::Win32::System::Com::CLSCTX_INPROC_SERVER).unwrap(),
            event_hook: windows::Win32::UI::Accessibility::SetWinEventHook(windows::Win32::UI::WindowsAndMessaging::EVENT_MIN, windows::Win32::UI::WindowsAndMessaging::EVENT_MAX, None, Some(Self::event_handler), 0, 0, windows::Win32::UI::WindowsAndMessaging::WINEVENT_OUTOFCONTEXT),
            hmonitor_default_layout_indices: std::collections::HashMap::new(),
            hwnd_locations: std::collections::HashMap::new(),
            workspaces: std::collections::HashMap::new(),
            foreground_hwnd: None,
            layouts: std::collections::HashMap::new(),
            window_settings: WindowSettings::default(),
            ignored_combinations: std::collections::HashSet::new(),
            ignored_hwnds: std::collections::HashSet::new(),
        }
            
    }

    pub unsafe fn initialize_monitors(&mut self) {

        let _ = windows::Win32::Graphics::Gdi::EnumDisplayMonitors(None, None, Some(Self::enum_display_monitors_callback), windows::Win32::Foundation::LPARAM(self as *mut WindowManager as isize));
        
    }

    // Note: it is required to call initialize_monitors() first
    pub unsafe fn initialize_with_layout(&mut self, default_layout: crate::layout::Layout) {

        for (hmonitor, layouts) in self.layouts.iter_mut() {

            let layout = match crate::layout::Layout::convert_for_monitor(&default_layout, windows::Win32::Graphics::Gdi::HMONITOR(*hmonitor)) {

                Some(val) => val,
                
                None => default_layout.clone(),
            
            };

            layouts.push(std::sync::Arc::new(std::sync::RwLock::new(layout)));

        }

        windows::Win32::UI::WindowsAndMessaging::EnumWindows(Some(Self::enum_windows_callback), windows::Win32::Foundation::LPARAM(self as *mut WindowManager as isize)).unwrap();

        let foreground_hwnd = windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow();

        if self.hwnd_locations.contains_key(&foreground_hwnd.0) {

            self.foreground_hwnd = Some(foreground_hwnd);

            self.set_border_to_focused(foreground_hwnd);

        }

        self.update();

    }

    pub unsafe fn set_default_layout(&mut self, hmonitor: windows::Win32::Graphics::Gdi::HMONITOR, mut layout: crate::layout::Layout) {

        match crate::layout::Layout::convert_for_monitor(&layout, hmonitor) {
            
            Some(new_layout) => layout = new_layout,
        
            None => (),
        
        }

        let layout_vec = self.layouts.get_mut(&hmonitor.0).unwrap();
        
        layout_vec.push(std::sync::Arc::new(std::sync::RwLock::new(layout)));

        *self.hmonitor_default_layout_indices.get_mut(&hmonitor.0).unwrap() = layout_vec.len() - 1;

    }

    pub fn set_default_layout_idx(&mut self, hmonitor: windows::Win32::Graphics::Gdi::HMONITOR, idx: usize) {

        *self.hmonitor_default_layout_indices.get_mut(&hmonitor.0).unwrap() = idx;

    }

    pub fn get_window_settings(&self) -> &WindowSettings {

        &self.window_settings

    }
    
    pub fn get_window_settings_mut(&mut self) -> &mut WindowSettings {

        &mut self.window_settings

    }

    pub unsafe fn window_created(&mut self, hwnd: windows::Win32::Foundation::HWND) {

        if self.ignored_hwnds.contains(&hwnd.0) {

            return;

        }

        let window_desktop_id;

        let monitor_id;

        let mut increment_after = None;

        match self.hwnd_locations.get(&hwnd.0) {

            Some((_, _, false, _)) => return,

            Some((guid, hmonitor, _, idx)) if !is_maximized(hwnd) => {

                window_desktop_id = *guid;

                monitor_id = *hmonitor;

                increment_after = Some(*idx);

                match self.workspaces.get_mut(&(window_desktop_id, monitor_id.0)) {

                    Some(workspace) => {

                        workspace.managed_window_handles.insert(*idx, hwnd);

                    },
                    
                    None => {

                        self.workspaces.insert((window_desktop_id, monitor_id.0), Workspace::new(hwnd, self.layouts.get(&monitor_id.0).unwrap()[*self.hmonitor_default_layout_indices.get(&monitor_id.0).unwrap()].clone()));
                        
                    },
                
                };

                self.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, false, *idx));

            },

            None => {

                let start_instant = std::time::Instant::now();

                'timeout: loop {
                    
                    while std::time::Instant::now() - start_instant < std::time::Duration::from_secs(1) {
                        
                        match self.virtual_desktop_manager.GetWindowDesktopId(hwnd) {

                            Ok(guid) if guid != windows::core::GUID::zeroed() => {

                                window_desktop_id = guid;

                                break 'timeout;

                            },

                            _ => continue,
                        }
                        
                    }

                    return;

                }

                monitor_id = windows::Win32::Graphics::Gdi::MonitorFromWindow(hwnd, windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONULL);

                if monitor_id.is_invalid() {

                    return;

                }

                match self.workspaces.get_mut(&(window_desktop_id, monitor_id.0)) {

                    Some(workspace) => {

                        if is_maximized(hwnd) {

                            self.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, true, workspace.managed_window_handles.len()));

                        }

                        else {

                            workspace.managed_window_handles.push(hwnd);

                            self.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, false, workspace.managed_window_handles.len() - 1));

                            increment_after = Some(workspace.managed_window_handles.len() - 1);
                        }

                    },
                    
                    None => {

                        if is_maximized(hwnd) {

                            self.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, true, 0));
                    
                        }

                        else {
                        
                            self.workspaces.insert((window_desktop_id, monitor_id.0), Workspace::new(hwnd, self.layouts.get(&monitor_id.0).unwrap()[*self.hmonitor_default_layout_indices.get(&monitor_id.0).unwrap()].clone()));

                            self.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, false, 0));

                            increment_after = Some(0);

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

    pub unsafe fn window_destroyed(&mut self, hwnd: windows::Win32::Foundation::HWND) {

        let location = match self.hwnd_locations.get(&hwnd.0) {

            Some(val) => val,

            None => {

                self.ignored_hwnds.remove(&hwnd.0);

                return;

            },

        };

        let window_desktop_id = location.0;

        let monitor_id = location.1;

        let flag = location.2;

        let idx = location.3;

        self.hwnd_locations.remove(&hwnd.0);

        if !flag {

            let workspace = self.workspaces.get_mut(&(window_desktop_id, monitor_id.0)).unwrap();

            workspace.managed_window_handles.remove(idx);

            for (guid, hmonitor, _, i) in self.hwnd_locations.values_mut() {

                if 
                    *guid == window_desktop_id && 
                    *hmonitor == monitor_id &&
                    *i > idx 
                {

                        *i -= 1;

                }

            }

        }

        self.update_workspace(window_desktop_id, monitor_id);

    }

    pub unsafe fn window_minimized_or_maximized(&mut self, hwnd: windows::Win32::Foundation::HWND) {

        let location = match self.hwnd_locations.get_mut(&hwnd.0) {

            Some((_, _, true, _)) | None => return,

            Some(val) => val,

        };

        let window_desktop_id = location.0;

        let monitor_id = location.1;

        let idx = location.3;

        let workspace = self.workspaces.get_mut(&(window_desktop_id, monitor_id.0)).unwrap();

        workspace.managed_window_handles.remove(idx);

        location.2 = true;

        for (guid, hmonitor, _, i) in self.hwnd_locations.values_mut() {

            if 
                *guid == window_desktop_id && 
                *hmonitor == monitor_id &&
                *i > idx 
            {

                    *i -= 1;

            }

        }

        self.update_workspace(window_desktop_id, monitor_id);

    }

    pub unsafe fn window_cloaked(&mut self, hwnd: windows::Win32::Foundation::HWND) {

        let location= match self.hwnd_locations.get(&hwnd.0) {
            
            Some(val) => val,
        
            None => return,
        
        };

        let old_window_desktop_id = location.0;

        let monitor_id = location.1;

        let new_window_desktop_id = match self.virtual_desktop_manager.GetWindowDesktopId(hwnd) {

            Ok(guid) if guid != old_window_desktop_id => guid,

            _ => return,

        };

        let flag = location.2;

        let old_idx = location.3;

        let new_idx;

        if !flag {

            let old_workspace = self.workspaces.get_mut(&(old_window_desktop_id, monitor_id.0)).unwrap();

            old_workspace.managed_window_handles.remove(old_idx);

            for (guid, hmonitor, _, i) in self.hwnd_locations.values_mut() {

                if 
                    *guid == old_window_desktop_id &&
                    *hmonitor == monitor_id &&
                    *i > old_idx
                {

                        *i -= 1;

                }

            }

            match self.workspaces.get_mut(&(new_window_desktop_id, monitor_id.0)) {

                Some(workspace) => {

                    workspace.managed_window_handles.push(hwnd);

                    new_idx = workspace.managed_window_handles.len() - 1;

                },

                None => {

                    self.workspaces.insert((new_window_desktop_id, monitor_id.0), Workspace::new(hwnd, self.layouts.get(&monitor_id.0).unwrap()[*self.hmonitor_default_layout_indices.get(&monitor_id.0).unwrap()].clone()));

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
    
    pub unsafe fn foreground_window_changed(&mut self, hwnd: windows::Win32::Foundation::HWND) {
    
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

        if !is_maximized(hwnd) {

            let location = self.hwnd_locations.get(&hwnd.0).unwrap();

            let window_desktop_id = location.0;

            let monitor_id = location.1;

            for (h, (guid, hmonitor, flag, _)) in self.hwnd_locations.iter_mut() {

                if 
                    *guid == window_desktop_id &&
                    *hmonitor == monitor_id &&
                    *flag &&
                    is_maximized(windows::Win32::Foundation::HWND(*h))
                {

                        let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(windows::Win32::Foundation::HWND(*h), windows::Win32::UI::WindowsAndMessaging::SW_MINIMIZE);

                }

            }

        }

    }

    pub unsafe fn window_move_finished(&mut self, hwnd: windows::Win32::Foundation::HWND) {

        let location = match self.hwnd_locations.get_mut(&hwnd.0) {

            Some(val) => val,

            None => return,

        };

        let window_desktop_id = location.0;

        let original_monitor_id = location.1;

        let flag = location.2;

        let idx = location.3;

        let new_monitor_id = windows::Win32::Graphics::Gdi::MonitorFromWindow(hwnd, windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONULL);

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

        let mut moved_to = windows::Win32::Foundation::RECT::default();

        windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut moved_to).unwrap();

        let moved_to_area = (moved_to.right - moved_to.left)*(moved_to.bottom - moved_to.top);

        let workspace;

        if changed_monitors {

            workspace = match self.workspaces.get_mut(&(window_desktop_id, new_monitor_id.0)) {

                Some(w) => w,

                None => {

                    self.workspaces.get_mut(&(window_desktop_id, original_monitor_id.0)).unwrap().managed_window_handles.remove(idx);

                    self.workspaces.insert((window_desktop_id, new_monitor_id.0), Workspace::new(hwnd, self.layouts.get(&new_monitor_id.0).unwrap()[*self.hmonitor_default_layout_indices.get(&new_monitor_id.0).unwrap()].clone()));

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

            let layout_read_lock = workspace.layout.read().unwrap();

            let positions = (*layout_read_lock).get(workspace.managed_window_handles.len() - 1);

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

    unsafe fn update_workspace(&mut self, guid: windows::core::GUID, hmonitor: windows::Win32::Graphics::Gdi::HMONITOR) {

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
        

        let mut len = (*workspace.layout.read().unwrap()).positions_len();

        while len < workspace.managed_window_handles.len() {
 
            (*workspace.layout.write().unwrap()).extend();

            len = (*workspace.layout.read().unwrap()).positions_len();

        }

        let mut error_indices: Option<Vec<usize>> = None;

        {

            let layout_read_lock = workspace.layout.try_read().unwrap();

            let positions = (*layout_read_lock).get(workspace.managed_window_handles.len() - 1);

            for (i, hwnd) in workspace.managed_window_handles.iter().enumerate() {

                match windows::Win32::UI::WindowsAndMessaging::SetWindowPos(*hwnd, None, positions[i].x, positions[i].y, positions[i].cx, positions[i].cy, windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER) {

                    Err(_) if windows::Win32::Foundation::GetLastError().0 == 5 => {

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
        
        }

        if let Some(v) = error_indices {

            for i in v {

                workspace.managed_window_handles.remove(i);

            }

            self.update_workspace(guid, hmonitor);

        }

    }

    unsafe fn update(&mut self) {

        let keys: Vec<(windows::core::GUID, *mut core::ffi::c_void)> = self.workspaces.keys().map(|k| (k.0, k.1)).collect();
        
        for k in keys.iter() {
            
            self.update_workspace(k.0, windows::Win32::Graphics::Gdi::HMONITOR(k.1));
        }

    }

    fn swap_windows(&mut self, guid: windows::core::GUID, hmonitor: windows::Win32::Graphics::Gdi::HMONITOR, i: usize, j: usize) {

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

    fn move_windows_across_monitors(&mut self, guid: windows::core::GUID, first_hmonitor: windows::Win32::Graphics::Gdi::HMONITOR, second_hmonitor: windows::Win32::Graphics::Gdi::HMONITOR, first_idx: usize, second_idx: usize) {

        let hwnd = self.workspaces.get_mut(&(guid, first_hmonitor.0)).unwrap().managed_window_handles.remove(first_idx);

        if !self.hwnd_locations.get(&hwnd.0).unwrap().2 {

            for (g, hmonitor, flag, i) in self.hwnd_locations.values_mut() {
                
                if
                    *g == guid &&
                    *hmonitor == first_hmonitor &&
                    *flag &&
                    *i > first_idx {

                        *i -= 1;

                }

            }

        }

        let location = self.hwnd_locations.get_mut(&hwnd.0).unwrap();

        self.workspaces.get_mut(&(guid, second_hmonitor.0)).unwrap().managed_window_handles.push(hwnd);

        let last_idx = self.workspaces.get(&(guid, second_hmonitor.0)).unwrap().managed_window_handles.len() - 1;

        location.1 = second_hmonitor;
        
        self.swap_windows(guid, second_hmonitor, second_idx, last_idx);



    }

    unsafe fn set_border_to_unfocused(&self, hwnd: windows::Win32::Foundation::HWND) {

        let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(hwnd, windows::Win32::Graphics::Dwm::DWMWA_BORDER_COLOR, &self.window_settings.get_unfocused_border_colour() as *const windows::Win32::Foundation::COLORREF as *const core::ffi::c_void, std::mem::size_of_val(&self.window_settings.get_unfocused_border_colour()) as u32);

    }

    unsafe fn set_border_to_focused(&self, hwnd: windows::Win32::Foundation::HWND) {

        let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(hwnd, windows::Win32::Graphics::Dwm::DWMWA_BORDER_COLOR, &self.window_settings.focused_border_colour as *const windows::Win32::Foundation::COLORREF as *const core::ffi::c_void, std::mem::size_of_val(&self.window_settings.focused_border_colour) as u32);

    }

    unsafe fn initialize_border(&self, hwnd: windows::Win32::Foundation::HWND) {
    
        let corner_preference = 

            if self.window_settings.disable_rounding {

                windows::Win32::Graphics::Dwm::DWMWCP_DONOTROUND

            }

            else {

                windows::Win32::Graphics::Dwm::DWMWCP_DEFAULT

            };

        let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(hwnd, windows::Win32::Graphics::Dwm::DWMWA_WINDOW_CORNER_PREFERENCE, &corner_preference as *const windows::Win32::Graphics::Dwm::DWM_WINDOW_CORNER_PREFERENCE as *const core::ffi::c_void, std::mem::size_of_val(&corner_preference) as u32);

        self.set_border_to_unfocused(hwnd);

    }

    unsafe extern "system" fn event_handler(_hwineventhook: windows::Win32::UI::Accessibility::HWINEVENTHOOK, event: u32, hwnd: windows::Win32::Foundation::HWND, idobject: i32, _idchild: i32, _ideventthread: u32, _dwmseventtime: u32) {

        if !has_sizebox(hwnd) {

            return;

        }

        match event {

            windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_SHOW if idobject == windows::Win32::UI::WindowsAndMessaging::OBJID_WINDOW.0 => {

                windows::Win32::UI::WindowsAndMessaging::PostMessageA(None, messages::WINDOW_CREATED, windows::Win32::Foundation::WPARAM(hwnd.0 as usize), None).unwrap();

            },

            windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_DESTROY if idobject == windows::Win32::UI::WindowsAndMessaging::OBJID_WINDOW.0 => {

                windows::Win32::UI::WindowsAndMessaging::PostMessageA(None, messages::WINDOW_DESTROYED, windows::Win32::Foundation::WPARAM(hwnd.0 as usize), None).unwrap();

            },

            windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_LOCATIONCHANGE => {

                if 
                    is_maximized(hwnd) || 
                    is_minimized(hwnd) 
                {

                    windows::Win32::UI::WindowsAndMessaging::PostMessageA(None, messages::WINDOW_MINIMIZED_OR_MAXIMIZED, windows::Win32::Foundation::WPARAM(hwnd.0 as usize), None).unwrap();

                }

                else {

                    windows::Win32::UI::WindowsAndMessaging::PostMessageA(None, messages::WINDOW_CREATED, windows::Win32::Foundation::WPARAM(hwnd.0 as usize), None).unwrap();

                }

            },
            
            windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_HIDE if idobject == windows::Win32::UI::WindowsAndMessaging::OBJID_WINDOW.0 => {

                windows::Win32::UI::WindowsAndMessaging::PostMessageA(None, messages::WINDOW_MINIMIZED_OR_MAXIMIZED, windows::Win32::Foundation::WPARAM(hwnd.0 as usize), None).unwrap();

            },

            windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_CLOAKED if idobject == windows::Win32::UI::WindowsAndMessaging::OBJID_WINDOW.0 => {

                windows::Win32::UI::WindowsAndMessaging::PostMessageA(None, messages::WINDOW_CLOAKED, windows::Win32::Foundation::WPARAM(hwnd.0 as usize), None).unwrap();

            },
        
            windows::Win32::UI::WindowsAndMessaging::EVENT_SYSTEM_FOREGROUND | windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_FOCUS => {

                windows::Win32::UI::WindowsAndMessaging::PostMessageA(None, messages::FOREGROUND_WINDOW_CHANGED, windows::Win32::Foundation::WPARAM(hwnd.0 as usize), None).unwrap();

            },

            windows::Win32::UI::WindowsAndMessaging::EVENT_SYSTEM_MOVESIZEEND => {

                windows::Win32::UI::WindowsAndMessaging::PostMessageA(None, messages::WINDOW_MOVE_FINISHED, windows::Win32::Foundation::WPARAM(hwnd.0 as usize), None).unwrap();

            },

            _ => return,

        }

    }

    unsafe extern "system" fn enum_windows_callback(hwnd: windows::Win32::Foundation::HWND, lparam: windows::Win32::Foundation::LPARAM) -> windows::Win32::Foundation::BOOL {

        let wm = &mut *(lparam.0 as *mut WindowManager);
        
        let window_desktop_id = match wm.virtual_desktop_manager.GetWindowDesktopId(hwnd) {

            Ok(guid) if guid != windows::core::GUID::zeroed() => guid,
            
            _ => return true.into(),

        };

        let monitor_id = windows::Win32::Graphics::Gdi::MonitorFromWindow(hwnd, windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONULL);

        if monitor_id.is_invalid() {

            return true.into();

        }

        if 
            !is_visible(hwnd) ||
            !has_sizebox(hwnd) ||
            is_minimized(hwnd)
        {
            
            return true.into();

        }

        match wm.workspaces.get_mut(&(window_desktop_id, monitor_id.0)) {

            Some(workspace) => {
                
                if is_maximized(hwnd) {

                    wm.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, true, workspace.managed_window_handles.len()));

                }

                else {

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

            },
            
            None => {

                if is_maximized(hwnd) {

                    wm.hwnd_locations.insert(hwnd.0, (window_desktop_id, monitor_id, true, 0));

                }

                else {


                    wm.workspaces.insert((window_desktop_id, monitor_id.0), Workspace::new(hwnd, wm.layouts.get(&monitor_id.0).unwrap()[*wm.hmonitor_default_layout_indices.get(&monitor_id.0).unwrap()].clone()));

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

            },
        
        }

        wm.initialize_border(hwnd);

        return true.into();

    }

    unsafe extern "system" fn enum_display_monitors_callback(hmonitor: windows::Win32::Graphics::Gdi::HMONITOR, _hdc: windows::Win32::Graphics::Gdi::HDC, _hdc_monitor: *mut windows::Win32::Foundation::RECT, dw_data: windows::Win32::Foundation::LPARAM) -> windows::Win32::Foundation::BOOL {

        let wm = &mut *(dw_data.0 as *mut WindowManager);

        wm.hmonitor_default_layout_indices.insert(hmonitor.0, 0);
        
        wm.layouts.insert(hmonitor.0, Vec::new());

        return true.into();

    }

}

unsafe fn is_maximized(hwnd: windows::Win32::Foundation::HWND) -> bool {

    windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrA(hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_STYLE) & windows::Win32::UI::WindowsAndMessaging::WS_MAXIMIZE.0 as isize != 0

}

unsafe fn is_minimized(hwnd: windows::Win32::Foundation::HWND) -> bool {

    windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrA(hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_STYLE) & windows::Win32::UI::WindowsAndMessaging::WS_MINIMIZE.0 as isize != 0

}

unsafe fn is_visible(hwnd: windows::Win32::Foundation::HWND) -> bool {

    windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrA(hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_STYLE) & windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE.0 as isize != 0

}

unsafe fn has_sizebox(hwnd: windows::Win32::Foundation::HWND) -> bool {

    windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrA(hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_STYLE) & windows::Win32::UI::WindowsAndMessaging::WS_SIZEBOX.0 as isize != 0

}