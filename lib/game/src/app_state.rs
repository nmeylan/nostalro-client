#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppState {
    Login,
    ServerSelect,
    CharacterSelect,
    InGame,
}
