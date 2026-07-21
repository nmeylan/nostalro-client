use crate::App;
use ragnarok_game::event::SelfConfigKind;

impl App {
    pub(super) fn handle_self_config_changed(&mut self, kind: SelfConfigKind, enabled: bool) {
        let config = &mut self.game.prefs.self_config;
        match kind {
            SelfConfigKind::RefusePartyInvite => config.refuse_party_invite = enabled,
            SelfConfigKind::OpenEquipmentWindow => config.open_equipment_window = enabled,
            SelfConfigKind::Call => config.call_enabled = enabled,
            SelfConfigKind::PetAutofeed => config.pet_autofeed = enabled,
            SelfConfigKind::HomunculusAutofeed => config.homun_autofeed = enabled,
        }
    }
}
