use serde::{

    Deserialize,

    Serialize

};

use windows::Win32::{

    Foundation::*, 

    Graphics::Gdi::*

};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SplitDirection {
    Horizontal(i32),
    Vertical(i32),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum EndTilingBehaviour {
    
    Directional {
        direction: Direction,
        start_from: usize,
        from_zones: Option<Vec<Zone>>,
        zone_idx: usize,
    },

    Repeating {
        splits: Vec<Vec<RepeatingSplit>>,
        zone_idx: usize,
    },

}

impl EndTilingBehaviour {
    
    pub fn default_directional() -> Self {
        
        EndTilingBehaviour::Directional {
            direction: Direction::Horizontal,
            start_from: 1,
            from_zones: None,
            zone_idx: 0,
        }

    }

    pub fn default_repeating() -> Self {
        
        EndTilingBehaviour::Repeating {
            splits: Vec::new(),
            zone_idx: 0,
        }

    }

}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RepeatingSplit {
    direction: Direction,
    split_ratio: f64,
    split_idx_offset: usize,
    swap: bool,
}

impl RepeatingSplit {

    pub fn new(direction: Direction, split_ratio: f64, split_idx_offset: usize, swap: bool) -> Self {

        RepeatingSplit {
            direction,
            split_ratio,
            split_idx_offset,
            swap,
        }

    }

}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Zone {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Zone {
    
    fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        
        Zone {
            left,
            top,
            right,
            bottom,
        }

    }

    fn from_rect(val: RECT) -> Self {

        Zone {
            left: val.left,
            top: val.top,
            right: val.right,
            bottom: val.bottom,
        }
        
    }

    fn w(&self) -> i32 {

        self.right - self.left
    
    }
    
    fn h(&self) -> i32 {
    
        self.bottom - self.top
    
    }

}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub cx: i32,
    pub cy: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Layout {
    monitor_rect: Zone,
    zones: Vec<Vec<Zone>>,
    manual_zones_until: usize,
    end_tiling_behaviour: EndTilingBehaviour,
    positions: Vec<Vec<Position>>,
}

impl Layout {
    
    fn new(w: i32, h: i32) -> Self {

        Layout {
            monitor_rect: Zone::new(0, 0, w, h),
            zones: vec![vec![Zone::new(0, 0, w, h)]],
            manual_zones_until: 1,
            end_tiling_behaviour: EndTilingBehaviour::default_directional(),
            positions: Vec::new(),
        }
    
    }
    
    pub fn delete_zones(&mut self, i: usize) {
        
        self.zones.remove(i);

    }

    pub fn swap_zone_vectors(&mut self, i: usize, j: usize) {
        
        if i == j {

            return;

        }

        let first_idx = std::cmp::min(i, j);

        let second_idx = std::cmp::max(i, j);

        let (first_slice, second_slice) = self.zones.split_at_mut(second_idx);

        std::mem::swap(&mut first_slice[first_idx], &mut second_slice[0]);

    }

    pub fn manual_zones_until(&self) -> usize {
    
        self.manual_zones_until

    }

    pub fn get(&self, i: usize) -> &Vec<Position> {

        &self.positions[i]
    
    }

    pub fn positions_len(&self) -> usize {

        self.positions.len()

    }

    pub fn update(&mut self, window_padding: i32, edge_padding: i32) {

        match &mut self.end_tiling_behaviour {
            
            EndTilingBehaviour::Directional { direction: _, start_from, from_zones, zone_idx: _ } if *start_from > 1 && matches!(from_zones, None) => {

                *from_zones = Some(self.zones.remove(self.zones.len() - 1));

                self.manual_zones_until -= 1;

            },
        
            _ => (),
        
        }

        self.positions = Vec::new();
        
        let mut len = 0;

        for zones in &self.zones {
            
            self.positions.push(Vec::new());

            len += 1;
            
            for zone in zones {

                let mut position = Position {
                    x: zone.left - 7 + window_padding ,
                    y: zone.top + window_padding,
                    cx: zone.w() + 14 - 2*window_padding,
                    cy: zone.h() + 7 - 2*window_padding,
                };

                if zone.left == self.monitor_rect.left {

                    position.x = position.x - window_padding + edge_padding;

                    position.cx = position.cx + window_padding - edge_padding;
                
                }

                if zone.top == self.monitor_rect.top {
                
                    position.y = position.y - window_padding + edge_padding;
                    
                    position.cy = position.cy + window_padding - edge_padding;
                
                }

                if zone.right == self.monitor_rect.right {
                
                    position.cx = position.cx + window_padding - edge_padding;
                
                }

                if zone.bottom == self.monitor_rect.bottom {
                
                    position.cy = position.cy + window_padding - edge_padding;
                
                }

                self.positions[len - 1].push(position);
            
            }

        }

    }

    pub fn set_end_tiling_behaviour(&mut self, behaviour: EndTilingBehaviour) {
        
        self.end_tiling_behaviour = behaviour;

    }

    pub fn get_end_zone_idx(&self) -> usize {

        match self.end_tiling_behaviour {

            EndTilingBehaviour::Directional { direction: _, start_from: _, from_zones: _, zone_idx } => return zone_idx,

            EndTilingBehaviour::Repeating { splits: _, zone_idx } => return zone_idx,
        
        }

    }

    pub fn set_end_zone_idx(&mut self, i: usize) {

        match &mut self.end_tiling_behaviour {

            EndTilingBehaviour::Directional { direction: _, start_from: _, from_zones: _, zone_idx } => {
                
                *zone_idx = i;
                    
            },

            EndTilingBehaviour::Repeating { splits: _, zone_idx } => {

                *zone_idx = i;

            },
            
        }

    }

    pub fn set_end_tiling_direction(&mut self, new_direction: Direction) {
        
        match &mut self.end_tiling_behaviour {

            EndTilingBehaviour::Directional { direction, start_from: _, from_zones: _, zone_idx: _ } => {

                *direction = new_direction;

            },
            
            EndTilingBehaviour::Repeating { splits: _, zone_idx: _ } => return,
        
        }

    }

    pub fn set_end_tiling_start_from(&mut self, val: usize) {
        
        match &mut self.end_tiling_behaviour {

            EndTilingBehaviour::Directional { direction: _, start_from, from_zones: _, zone_idx: _ } => {

                *start_from = val;

            },
            
            EndTilingBehaviour::Repeating { splits: _, zone_idx: _ } => return,
        
        }

    }

    pub fn add_repeating_split(&mut self, direction: Direction, split_ratio: f64, split_idx_offset: usize, swap: bool) {

        if let EndTilingBehaviour::Repeating { splits, zone_idx: _ } = &mut self.end_tiling_behaviour {

            if splits.len() == 0 {

                splits.push(vec![RepeatingSplit::new(direction, split_ratio, split_idx_offset, swap)]);

            }

            else {

                splits.push(splits[splits.len() - 1].clone());

                let idx = splits.len() - 1;

                splits[idx].push(RepeatingSplit::new(direction, split_ratio, split_idx_offset, swap));

            }

        }

    }

    pub fn remove_repeating_split(&mut self, i: usize, j: usize) {
        
        if let EndTilingBehaviour::Repeating { splits, zone_idx: _ } = &mut self.end_tiling_behaviour {
            
            splits[i].remove(j);

        }

    }

    pub fn set_repeating_split_direction(&mut self, i: usize, j: usize, direction: Direction) {

        if let EndTilingBehaviour::Repeating { splits, zone_idx: _ } = &mut self.end_tiling_behaviour {

            splits[i][j].direction = direction;

        }

    }

    pub fn set_repeating_split_ratio(&mut self, i: usize, j: usize, val: f64) {

        if let EndTilingBehaviour::Repeating { splits, zone_idx: _ } = &mut self.end_tiling_behaviour {

            splits[i][j].split_ratio = val;

        }

    }

    pub fn set_repeating_split_idx_offset(&mut self, i: usize, j: usize, val: usize) {

        if let EndTilingBehaviour::Repeating { splits, zone_idx: _ } = &mut self.end_tiling_behaviour {

            splits[i][j].split_idx_offset = val;

        }

    }

    pub fn set_repeating_split_swap(&mut self, i: usize, j: usize, val: bool) {

        if let EndTilingBehaviour::Repeating { splits, zone_idx: _ } = &mut self.end_tiling_behaviour {

            splits[i][j].swap = val;

        }

    }

    pub fn new_zone_vec(&mut self) {

        self.zones.push(vec![Zone::new(self.monitor_rect.left, self.monitor_rect.top, self.monitor_rect.right, self.monitor_rect.bottom)]);

        self.manual_zones_until += 1;

    }

    pub fn new_zone_vec_from(&mut self, i: usize) {
        
        self.zones.push(self.zones[i].clone());

        self.manual_zones_until += 1;

    }

    pub fn split(&mut self, i: usize, j: usize, direction: SplitDirection) {

        let zone = &mut self.zones[i][j];

        let new_zone;

        match direction {
            
            SplitDirection::Horizontal(at) => {
                
                if at - zone.top < zone.h()/2 {
                
                    new_zone = Zone::new(zone.left, zone.top, zone.right, at);

                    zone.top = at;

                }

                else {

                    new_zone = Zone::new(zone.left, at, zone.right, zone.bottom);

                    zone.bottom = at;

                }

            },
            
            SplitDirection::Vertical(at) => {

                if at - zone.left < zone.w()/2 {

                    new_zone = Zone::new(zone.left, zone.top, at, zone.bottom);

                    zone.left = at;
                
                }

                else {

                    new_zone = Zone::new(at, zone.top, zone.right, zone.bottom);
                    
                    zone.right = at;

                }

            },
        }

        self.set_end_zone_idx(self.zones[i].len());

        self.zones[i].push(new_zone);

    }

    pub fn can_merge_zones(&self, i: usize, j: usize, k: usize) -> bool {

        if j == k {

            return false;

        }

        let first_zone = &self.zones[i][j];

        let second_zone = &self.zones[i][k];

        return
            
            (
                
                first_zone.left == second_zone.left &&
                first_zone.right == second_zone.right
                
                &&

                (

                    first_zone.bottom == second_zone.top ||
                    first_zone.top == second_zone.bottom

                )
                
            )

            ||

            (

                first_zone.top == second_zone.top &&
                first_zone.bottom == second_zone.bottom

                &&

                (

                    first_zone.right == second_zone.left ||
                    first_zone.left == second_zone.right

                )

            );
        
    }

    pub fn merge_zones(&mut self, i: usize, j: usize, k: usize) {

        if j == k {

            return;

        }

        let first_idx = std::cmp::min(j, k);

        let second_idx = std::cmp::max(j, k);

        if 
            self.zones[i][first_idx].left == self.zones[i][second_idx].left &&
            self.zones[i][first_idx].right == self.zones[i][second_idx].right
        {

            if self.zones[i][first_idx].bottom == self.zones[i][second_idx].top {

                self.zones[i][first_idx].bottom = self.zones[i][second_idx].bottom;

            }

            else if self.zones[i][first_idx].top == self.zones[i][second_idx].bottom {

                self.zones[i][first_idx].top = self.zones[i][second_idx].top;

            }

        }

        else if 
            self.zones[i][first_idx].top == self.zones[i][second_idx].top &&
            self.zones[i][first_idx].bottom == self.zones[i][second_idx].bottom
        {

            if self.zones[i][first_idx].right == self.zones[i][second_idx].left {

                self.zones[i][first_idx].right = self.zones[i][second_idx].right;

            }

            else if self.zones[i][first_idx].left == self.zones[i][second_idx].right {

                self.zones[i][first_idx].left = self.zones[i][second_idx].left;

            }

        }

        self.zones[i].remove(second_idx);

    }

    pub fn swap_zones(&mut self, i: usize, j: usize, k: usize) {
        
        if j == k {

            return;

        }

        let first_idx = std::cmp::min(j, k);

        let second_idx = std::cmp::max(j, k);

        let (first_slice, second_slice) = self.zones[i].split_at_mut(second_idx);
        
        std::mem::swap(&mut first_slice[first_idx], &mut second_slice[0]);

    }

    pub fn extend(&mut self) {

        let end_zone_idx = self.get_end_zone_idx();

        let end_tiling_behaviour = self.end_tiling_behaviour.clone();
        
        match end_tiling_behaviour {

            EndTilingBehaviour::Directional { direction, start_from, from_zones, zone_idx } => {

                match start_from {

                    1 => {

                        self.zones.push(self.zones[self.manual_zones_until - 1].clone());
                    
                    },
                    
                    _ => {

                        self.zones.push(from_zones.unwrap().clone());

                    },

                }

                match direction {

                    Direction::Horizontal => {
                        
                        let offset = (self.zones[self.zones.len() - 1][zone_idx].h())/(self.zones.len() - self.zones[self.zones.len() - 1].len() + 1) as i32;

                        while self.zones[self.zones.len() - 1].len() < self.zones.len() {

                            self.split(self.zones.len() - 1, zone_idx, SplitDirection::Horizontal(self.zones[self.zones.len() - 1][zone_idx].top + offset));

                            self.set_end_zone_idx(end_zone_idx);
                        
                        }

                    },
                    
                    Direction::Vertical => {
                        
                        let offset = (self.zones[self.zones.len() - 1][zone_idx].w())/(self.zones.len() - self.zones[self.zones.len() - 1].len() + 1) as i32;

                        while self.zones[self.zones.len() - 1].len() < self.zones.len() {
                            
                            self.split(self.zones.len() - 1, zone_idx, SplitDirection::Vertical(self.zones[self.zones.len() - 1][zone_idx].left + offset));

                            self.set_end_zone_idx(end_zone_idx);

                        }

                    },
                
                }

            },

            EndTilingBehaviour::Repeating { splits, zone_idx } => {

                let repeating_idx = (self.zones.len() - self.manual_zones_until) % splits.len();

                self.zones.push(self.zones[self.zones.len() - 1 - ((self.zones.len() - self.manual_zones_until) % splits.len())].clone());

                for (i, split) in splits[repeating_idx].iter().enumerate() {

                    let split_idx = match i {

                        0 if (self.zones.len() - 1 - self.manual_zones_until)/splits.len() == 0 => {

                            zone_idx

                        },

                        0 => {

                            self.zones[self.zones.len() - 1].len() - 1 - splits.len() + split.split_idx_offset

                        },

                        _ => {

                            self.zones[self.zones.len() - 1].len() - 1 - i + split.split_idx_offset

                        },

                    };

                    let at;

                    match split.direction {

                        Direction::Horizontal => {

                            at = self.zones[self.zones.len() - 1][split_idx].top + (split.split_ratio*(self.zones[self.zones.len() - 1][split_idx].h() as f64)).round() as i32;

                            self.split(self.zones.len() - 1, split_idx, SplitDirection::Horizontal(at));

                        },
                        
                        Direction::Vertical => {
                            
                            at = self.zones[self.zones.len() - 1][split_idx].left + (split.split_ratio*(self.zones[self.zones.len() - 1][split_idx].w() as f64)).round() as i32;

                            self.split(self.zones.len() - 1, split_idx, SplitDirection::Vertical(at));

                        },

                    }

                    self.set_end_zone_idx(end_zone_idx);

                    if split.swap {

                        self.swap_zones(self.zones.len() - 1, split_idx, self.zones[self.zones.len() - 1].len() - 1);

                    }

                }

            },
        
        }

    }

}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LayoutGroup {
    layouts: Vec<Layout>,
    default_idx: usize,
}

impl LayoutGroup {

    pub fn new(w: i32, h: i32) -> Self {

        LayoutGroup {
            layouts: vec![Layout::new(w, h)],
            default_idx: 0,
        }

    }

    pub fn default_idx(&self) -> usize {

        self.default_idx

    }

    pub fn set_default_idx(&mut self, i: usize) {

        self.default_idx = i;

    }

    pub fn get_layouts(&self) -> &Vec<Layout> {

        &self.layouts

    }

    pub fn get_layouts_mut(&mut self) -> &mut Vec<Layout> {

        &mut self.layouts

    }

    pub fn layouts_len(&self) -> usize {

        self.layouts.len()

    }

    pub fn update_all(&mut self, window_padding: i32, edge_padding: i32) {

        for layout in self.layouts.iter_mut() {

            layout.update(window_padding, edge_padding);

        }

    }

    pub unsafe fn convert_for_monitor(layout_group: &LayoutGroup, hmonitor: HMONITOR) -> Option<LayoutGroup> {

        let mut monitor_info = MONITORINFO::default();

        monitor_info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;

        let _ = GetMonitorInfoA(hmonitor, &mut monitor_info);
        
        let monitor_rect = Zone::from_rect(monitor_info.rcWork);

        let layout = &layout_group.layouts[layout_group.default_idx];

        if monitor_rect == layout.monitor_rect {

            return None;

        }

        let original_width = (layout.monitor_rect.right - layout.monitor_rect.left) as f64;

        let original_height = (layout.monitor_rect.bottom - layout.monitor_rect.top) as f64 ;

        let new_width = (monitor_rect.right - monitor_rect.left) as f64;
        
        let new_height = (monitor_rect.bottom - monitor_rect.top) as f64;

        let mut ret = layout_group.clone();

        for l in ret.layouts.iter_mut() {

            for zones in l.zones.iter_mut() {

                for zone in zones {

                    zone.left -= layout.monitor_rect.left;

                    zone.top -= layout.monitor_rect.top;

                    zone.right -= layout.monitor_rect.left;

                    zone.bottom -= layout.monitor_rect.top;

                    if new_width != original_width {

                        zone.left = ((zone.left as f64*new_width)/original_width).round() as i32;
                        
                        zone.right = ((zone.right as f64*new_width)/original_width).round() as i32;

                    }

                    if new_height != original_height {

                        zone.top = ((zone.top as f64*new_height)/original_height).round() as i32;

                        zone.bottom = ((zone.bottom as f64*new_height)/original_height).round() as i32;

                    }
                    
                    zone.left += (&monitor_rect).left;

                    zone.top += (&monitor_rect).top;

                    zone.right += (&monitor_rect).left;

                    zone.bottom += (&monitor_rect).top;

                }

            }

            l.monitor_rect = monitor_rect.clone();

        }

        return Some(ret);
        
    }
    
    pub fn new_variant(&mut self) {
        
        self.layouts.push(self.layouts[self.default_idx].clone());

    }

    pub fn swap_variants(&mut self, i: usize, j: usize) {

        if i == j {

            return;

        }

        if self.default_idx == i {

            self.default_idx = j;

        }

        else if self.default_idx == j {

            self.default_idx = i;

        }

        let first_idx = std::cmp::min(i, j);

        let second_idx = std::cmp::max(i, j);

        let (first_slice, second_slice) = self.layouts.split_at_mut(second_idx);

        std::mem::swap(&mut first_slice[first_idx], &mut second_slice[0]);

    }

}
