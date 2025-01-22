use directories::BaseDirs;

use himewm_layout::*;

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

pub fn create_dirs() -> std::io::Result<()> {
    
    let dirs = Directories::new();

    let _config_dir = std::fs::create_dir(dirs.config_dir)?;

    let _layouts_dir = std::fs::create_dir(dirs.layouts_dir)?;

    return Ok(());

}

pub fn initialize_settings() -> himewm::Settings {
    
    let dirs = Directories::new();

    match std::fs::read(dirs.config_dir.join("settings.json")) {
        
        Ok(byte_vector) => {

            match serde_json::from_slice::<himewm::Settings>(byte_vector.as_slice()) {

                Ok(settings) => {
                    
                    return settings;

                },

                Err(_) => {

                    return himewm::Settings::default();

                },

            }

        },
    
        Err(_) => {

            let settings_file = std::fs::File::create_new(dirs.config_dir.join("settings.json")).unwrap();

            let default_settings = himewm::Settings::default();

            let _ = serde_json::to_writer_pretty(settings_file, &default_settings);

            return default_settings;


        },
    
    }

}

pub fn initialize_layouts() -> Option<Vec<LayoutGroup>> {
    
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

                        ret.push(layout_group);

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
