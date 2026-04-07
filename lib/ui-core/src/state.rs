use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::frame::{WidgetId, WindowState};

pub struct StateCache {
    map: HashMap<(WidgetId, TypeId), Box<dyn Any>>,
}

impl StateCache {
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    pub fn get_or_default<T: Any + Default>(&mut self, id: WidgetId) -> &mut T {
        self.map
            .entry((id, TypeId::of::<T>()))
            .or_insert_with(|| Box::new(T::default()))
            .downcast_mut::<T>()
            .expect("type mismatch in StateCache")
    }

    pub fn get<T: Any>(&self, id: WidgetId) -> Option<&T> {
        self.map
            .get(&(id, TypeId::of::<T>()))
            .and_then(|b| b.downcast_ref::<T>())
    }

    pub fn set<T: Any>(&mut self, id: WidgetId, value: T) {
        self.map.insert((id, TypeId::of::<T>()), Box::new(value));
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn extract_window_positions(&self) -> HashMap<u32, [f32; 2]> {
        let ws_type = TypeId::of::<WindowState>();
        self.map.iter()
            .filter_map(|((wid, tid), val)| {
                if *tid == ws_type {
                    val.downcast_ref::<WindowState>().map(|ws| (wid.0, [ws.x, ws.y]))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_persists_across_calls() {
        let mut cache = StateCache::new();
        let id = WidgetId(1);

        *cache.get_or_default::<f32>(id) = 42.0;
        assert_eq!(*cache.get_or_default::<f32>(id), 42.0);
    }

    #[test]
    fn different_types_on_same_id_are_independent() {
        let mut cache = StateCache::new();
        let id = WidgetId(1);

        cache.set::<f32>(id, 3.14);
        cache.set::<String>(id, "hello".into());

        assert_eq!(cache.get::<f32>(id), Some(&3.14));
        assert_eq!(cache.get::<String>(id), Some(&"hello".to_string()));
    }
}
