use std::collections::HashMap;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table::{self, QuestDisplay};

const PATH: &str = "data/questid2display.txt";
const ITEM_TEX_DIR: &str = "data/texture/유저인터페이스/item";
const DEFAULT_ICON: &str = "sg_feel";
const DEFAULT_IMAGE: &str = "que_noimage";

pub struct QuestDisplayTable {
    entries: HashMap<u32, QuestDisplay>,
}

impl QuestDisplayTable {
    pub fn from_entries(entries: HashMap<u32, QuestDisplay>) -> Self {
        Self { entries }
    }

    pub fn load(grf: &GrfArchive) -> Self {
        let entries = grf
            .read_file(PATH)
            .map(|data| lua_table::parse_questid2display(&data))
            .unwrap_or_default();
        tracing::info!("Loaded quest display table: {} quests", entries.len());
        Self { entries }
    }

    pub fn get(&self, id: u32) -> Option<&QuestDisplay> {
        self.entries.get(&id)
    }

    pub fn title(&self, id: u32) -> String {
        self.entries
            .get(&id)
            .filter(|q| !q.title.is_empty())
            .map(|q| q.title.clone())
            .unwrap_or_else(|| "Unknown Quest".to_string())
    }

    pub fn summary(&self, id: u32) -> String {
        self.entries
            .get(&id)
            .map(|q| q.summary.clone())
            .unwrap_or_default()
    }

    pub fn description(&self, id: u32) -> String {
        self.entries
            .get(&id)
            .map(|q| q.description.clone())
            .unwrap_or_default()
    }

    /// Icon texture path. Every record in our data is SG_FEEL; the newer `ico_*`
    /// names map back to it.
    pub fn icon_texture(&self, id: u32) -> String {
        let name = self
            .entries
            .get(&id)
            .map(|q| q.icon_name.to_ascii_lowercase())
            .filter(|n| !n.is_empty() && !n.starts_with("ico"))
            .unwrap_or_else(|| DEFAULT_ICON.to_string());
        format!("{ITEM_TEX_DIR}/{name}.bmp")
    }

    pub fn image_texture(&self, id: u32) -> String {
        let name = self
            .entries
            .get(&id)
            .map(|q| q.image_name.to_ascii_lowercase())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| DEFAULT_IMAGE.to_string());
        format!("{ITEM_TEX_DIR}/{name}.bmp")
    }
}
