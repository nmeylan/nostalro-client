use crate::App;

impl App {
    pub(super) fn handle_adoption_requested(
        &mut self,
        father_aid: u32,
        mother_aid: u32,
        name: String,
    ) {
        self.game.pending_confirms.pending_adopt_request = Some((father_aid, mother_aid));
        self.game.pending_confirms.adopt_request_result.set(None);
        let msg = format!("{name} wishes to adopt you. Do you accept?");
        self.game.confirm_dialog.show_with_out(
            &msg,
            true,
            self.game.pending_confirms.adopt_request_result.clone(),
            |_| {},
        );
    }

    pub(super) fn handle_adoption_message(&mut self, msg_no: i32) {
        let text = match msg_no {
            0 => "You cannot adopt more than 1 child.",
            1 => "You must be at least character level 70 in order to adopt someone.",
            2 => "You cannot adopt a married person.",
            _ => "Adoption failed.",
        };
        self.game.chat_window.add_system(text.to_string());
    }
}
