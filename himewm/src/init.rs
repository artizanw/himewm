use directories::BaseDirs;

use himewm_layout::*;

use serde::{

    Deserialize,

    Serialize

};

use windows::Win32::Foundation::COLORREF;

struct Directories {
    config_dir: std::path::PathBuf,
    layouts_dir: std::path::PathBuf,
}

impl Directories {

    fn new() -> Self {
        
        let base_dirs = BaseDirs::new().unwrap();

        let config_dir = base_dirs.config_dir().join("himewm");

        let layouts_dir = config_dir.join("layouts");

        return 
            
            Directories {
                config_dir,
                layouts_dir,
            }
        
    }

}

#[derive(Deserialize, Serialize)]
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

#[derive(Deserialize, Serialize)]
pub struct UserSettings {
    default_layout: std::path::PathBuf,
    window_padding: i32,
    edge_padding: i32,
    disable_rounding: bool,
    disable_unfocused_border: bool,
    focused_border_colour: Colour,
}

impl Default for UserSettings {

    fn default() -> Self {

        UserSettings {
            default_layout: std::path::PathBuf::new(),
            window_padding: 0,
            edge_padding: 0,
            disable_rounding: false,
            disable_unfocused_border: false,
            focused_border_colour: Colour { r: 255, g: 255, b: 255 },
        }

    }

}

impl UserSettings {
    
    pub fn to_settings(&self, layouts: &Vec<(std::path::PathBuf, himewm_layout::LayoutGroup)>) -> himewm::Settings {

        if self.default_layout != std::path::Path::new("") {

            for (idx, (p, _)) in layouts.iter().enumerate() {

                if p == &self.default_layout {

                    return himewm::Settings {
                        default_layout_idx: idx,
                        window_padding: self.window_padding,
                        edge_padding: self.edge_padding,
                        disable_rounding: self.disable_rounding,
                        disable_unfocused_border: self.disable_unfocused_border,
                        focused_border_colour: self.focused_border_colour.as_colorref(),
                    };


                }

            }

        }

        return himewm::Settings {
            default_layout_idx: 0,
            window_padding: self.window_padding,
            edge_padding: self.edge_padding,
            disable_rounding: self.disable_rounding,
            disable_unfocused_border: self.disable_unfocused_border,
            focused_border_colour: self.focused_border_colour.as_colorref(),
        };

    }

}

pub fn create_dirs() -> std::io::Result<()> {
    
    let dirs = Directories::new();

    let _config_dir = std::fs::create_dir(dirs.config_dir)?;

    let _layouts_dir = std::fs::create_dir(dirs.layouts_dir)?;

    return Ok(());

}

pub fn initialize_settings() -> UserSettings {
    
    let dirs = Directories::new();

    match std::fs::read(dirs.config_dir.join("settings.json")) {
        
        Ok(byte_vector) => {

            match serde_json::from_slice::<UserSettings>(byte_vector.as_slice()) {

                Ok(settings) => {
                    
                    return settings;

                },

                Err(_) => {

                    return UserSettings::default();

                },

            }

        },
    
        Err(_) => {

            let settings_file = std::fs::File::create_new(dirs.config_dir.join("settings.json")).unwrap();

            let default_user_settings = UserSettings::default();

            let _ = serde_json::to_writer_pretty(settings_file, &default_user_settings);

            return default_user_settings;


        },
    
    }

}

pub fn initialize_layouts() -> Option<Vec<(std::path::PathBuf, LayoutGroup)>> {
    
    let mut ret = Vec::new();

    let dirs = Directories::new();
    
    for entry_result in std::fs::read_dir(dirs.layouts_dir).unwrap() {

        match entry_result {

            Ok(entry) => {

                match std::fs::read(entry.path()) {
                    
                    Ok(byte_vector) => {

                        let layout_group: LayoutGroup = match serde_json::from_slice(byte_vector.as_slice()) {
                            
                            Ok(val) => val,
                        
                            Err(_) => continue,
                        
                        };
                        
                        let layout_name = std::path::Path::new(&entry.file_name()).with_extension("");

                        ret.push((layout_name, layout_group));

                    },
                
                    Err(_) => continue,
                
                }

            },
    
            Err(_) => continue,
        
        }

    }

    if ret.is_empty() {

        return None;

    }

    else {

        return Some(ret);

    }

}
