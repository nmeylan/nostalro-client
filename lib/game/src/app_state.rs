#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppState {
    LoginServerSelect,
    Login,
    ServerSelect,
    CharacterSelect,
    CharacterCreate,
    InGame,
}
