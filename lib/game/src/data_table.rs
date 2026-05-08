use crate::accessory_table::AccessoryTable;
use crate::card_illustration_table::CardIllustrationTable;
use crate::card_name_table::CardNameTable;
use crate::item_description_table::ItemDescriptionTable;
use crate::item_name_table::ItemNameTable;
use crate::item_resource_table::ItemResourceTable;
use crate::item_slot_count_table::ItemSlotCountTable;
use crate::name_table::NameTable;
use crate::skill_description_table::SkillDescriptionTable;
use crate::skill_name_table::SkillNameTable;
use crate::skill_tree_table::SkillTreeTable;
use crate::skill_use_level_table::SkillUseLevelTable;

#[derive(Default)]
pub struct DataTable {
    pub name: Option<NameTable>,
    pub accessory: Option<AccessoryTable>,
    pub item_name: Option<ItemNameTable>,
    pub item_resource: Option<ItemResourceTable>,
    pub item_slot_count: Option<ItemSlotCountTable>,
    pub card_name: Option<CardNameTable>,
    pub card_illustration: Option<CardIllustrationTable>,
    pub item_description: Option<ItemDescriptionTable>,
    pub skill_name: Option<SkillNameTable>,
    pub skill_description: Option<SkillDescriptionTable>,
    pub skill_tree: Option<SkillTreeTable>,
    pub skill_use_level: Option<SkillUseLevelTable>,
}

impl DataTable {

    pub fn new() -> Self {
        Self {
            name: None,
            accessory: None,
            item_name: None,
            item_resource: None,
            item_slot_count: None,
            card_name: None,
            card_illustration: None,
            item_description: None,
            skill_name: None,
            skill_description: None,
            skill_tree: None,
            skill_use_level: None,
        }
    }
}
