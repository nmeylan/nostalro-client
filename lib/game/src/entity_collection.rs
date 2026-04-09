use std::collections::HashMap;

use crate::entity::Entity;

pub struct EntityCollection {
    entities: HashMap<u32, Entity>,
    player_id: Option<u32>,
}

impl EntityCollection {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            player_id: None,
        }
    }

    pub fn set_player_id(&mut self, id: u32) {
        self.player_id = Some(id);
    }

    pub fn player_id(&self) -> Option<u32> {
        self.player_id
    }

    pub fn player(&self) -> Option<&Entity> {
        self.player_id.and_then(|id| self.entities.get(&id))
    }

    pub fn player_mut(&mut self) -> Option<&mut Entity> {
        self.player_id.and_then(|id| self.entities.get_mut(&id))
    }

    pub fn insert(&mut self, entity: Entity) {
        self.entities.insert(entity.id, entity);
    }

    pub fn remove(&mut self, id: u32) -> Option<Entity> {
        self.entities.remove(&id)
    }

    pub fn get(&self, id: u32) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Entity> {
        self.entities.values_mut()
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        self.player_id = None;
    }

    pub fn is_player(&self, id: u32) -> bool {
        self.player_id == Some(id)
    }

    pub fn clear_non_player(&mut self) {
        if let Some(pid) = self.player_id {
            self.entities.retain(|&id, _| id == pid);
        } else {
            self.entities.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityType;

    fn make_entity(id: u32) -> Entity {
        Entity::new(id, EntityType::Player, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 150)
    }

    #[test]
    fn insert_get_remove_and_player() {
        let mut col = EntityCollection::new();
        col.set_player_id(100);
        col.insert(make_entity(100));
        col.insert(make_entity(200));
        col.insert(make_entity(300));

        assert!(col.player().is_some());
        assert_eq!(col.player().unwrap().id, 100);
        assert!(col.get(200).is_some());
        assert!(col.get(999).is_none());

        col.remove(200);
        assert!(col.get(200).is_none());

        col.clear_non_player();
        assert!(col.player().is_some());
        assert!(col.get(300).is_none());
    }
}
