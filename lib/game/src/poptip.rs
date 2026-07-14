const SOLID_SECS: f32 = 3.0;
const FADE_SECS: f32 = 1.0;
const MAX_ITEMS: usize = 2;

struct PoptipItem {
    text: String,
    age: f32,
}

#[derive(Default)]
pub struct PoptipStack {
    items: Vec<PoptipItem>,
}

impl PoptipStack {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(&mut self, text: String) {
        self.items.retain(|i| i.text != text);
        if self.items.len() >= MAX_ITEMS {
            self.items.pop();
        }
        self.items.insert(0, PoptipItem { text, age: 0.0 });
    }

    pub fn tick(&mut self, dt: f32) {
        for item in &mut self.items {
            item.age += dt;
        }
        self.items.retain(|i| i.age < SOLID_SECS + FADE_SECS);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Newest first. Yields `(text, alpha)`; alpha ramps from 1 to 0 during the
    /// fade window after the solid period.
    pub fn iter(&self) -> impl Iterator<Item = (&str, f32)> {
        self.items.iter().map(|item| {
            let alpha = if item.age <= SOLID_SECS {
                1.0
            } else {
                (1.0 - (item.age - SOLID_SECS) / FADE_SECS).clamp(0.0, 1.0)
            };
            (item.text.as_str(), alpha)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poptip_dedupes_caps_and_expires() {
        let mut stack = PoptipStack::new();
        stack.push("Server maintenance".to_string());
        stack.push("Server maintenance".to_string());
        assert_eq!(stack.iter().count(), 1);

        stack.push("Event started".to_string());
        stack.push("Boss spawned".to_string());
        assert_eq!(stack.iter().count(), MAX_ITEMS);
        assert_eq!(stack.iter().next().map(|(t, _)| t), Some("Boss spawned"));

        stack.tick(SOLID_SECS + FADE_SECS + 0.1);
        assert!(stack.is_empty());
    }
}
