pub mod attenuation;
pub mod decode;
pub mod manager;
pub mod mixer;

pub use attenuation::{DEFAULT_MAX_DIST, DEFAULT_MIN_DIST, attenuate};
pub use decode::Pcm;
pub use manager::SoundManager;
