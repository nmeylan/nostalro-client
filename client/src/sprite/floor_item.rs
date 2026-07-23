use crate::{App, input};
use ragnarok_game::cursor::{RenderEntry, RenderEntryKind};
use ragnarok_game::sprite_loader;
use std::rc::Rc;

impl App {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_floor_item_appeared(
        &mut self,
        id: u32,
        item_id: u16,
        is_identified: bool,
        x: i16,
        y: i16,
        sub_x: u8,
        sub_y: u8,
        count: i16,
        is_falling: bool,
    ) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let name = self
            .game
            .data_table
            .item_name
            .as_ref()
            .map(|t| t.get_name_or_id_for(item_id, is_identified))
            .unwrap_or_else(|| format!("Item #{item_id}"));
        let resource_name = self
            .game
            .data_table
            .item_resource
            .as_ref()
            .and_then(|t| t.get_resource_name_for(item_id, is_identified))
            .map(|s| s.to_string());

        let cell_x = x as f32 + sub_x as f32 / 16.0;
        let cell_y = y as f32 + sub_y as f32 / 16.0;
        let ground_y = self
            .game
            .session
            .gat
            .as_ref()
            .map(|gat| gat.get_height(cell_x + 0.5, cell_y + 0.5))
            .unwrap_or(0.0);

        let floor_item = ragnarok_game::floor_item::FloorItem {
            id,
            item_id,
            is_identified,
            x,
            y,
            sub_x,
            sub_y,
            count,
            name,
            resource_name: resource_name.clone(),
            drop_time: elapsed,
            is_falling,
            initial_y: ground_y,
        };
        self.game.world.floor_items.insert(id, floor_item);

        if let Some(res_name) = &resource_name
            && let Some(grf) = &self.grf
        {
            let base = format!("data/sprite/아이템/{res_name}");
            let spr_path = format!("{base}.spr");
            let act_path = format!("{base}.act");
            if let Some(data) = sprite_loader::load_sprite_data(grf, &spr_path, &act_path)
                && let Some(tex) = self.upload_sprite(&data)
            {
                self.game
                    .assets
                    .floor_item_sprites
                    .insert(id, (Rc::new(tex), data.act));
            }
        }
    }

    pub(crate) fn compute_floor_item_render_list(&self) -> Vec<RenderEntry> {
        let mut render_list = Vec::new();
        if let Some((renderer, coords, screen_w, screen_h)) = self.screen_dims() {
            for floor_item in self.game.world.floor_items.values() {
                let projected = input::entity_screen_params(
                    floor_item.world_position(),
                    self.game.session.gat.as_ref(),
                    coords,
                    &renderer.camera,
                    screen_w,
                    screen_h,
                );
                Self::push_projected(
                    &mut render_list,
                    RenderEntryKind::FloorItem,
                    floor_item.id,
                    projected,
                    None,
                    Some(0),
                    |screen_anchor, _, _, sprite_scale| {
                        let half = 17.0 * sprite_scale;
                        (
                            [
                                screen_anchor[0] - half,
                                screen_anchor[1] - half,
                                screen_anchor[0] + half,
                                screen_anchor[1] + half,
                            ],
                            half * 2.0,
                        )
                    },
                );
            }
        }
        render_list.sort_by(|a, b| {
            b.depth
                .partial_cmp(&a.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        render_list
    }
}
