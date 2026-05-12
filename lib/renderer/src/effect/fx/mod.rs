//! Concrete custom-effect implementations grouped by primitive family.
//!
//! Each module here implements [`super::CustomEffect`] for one family (or
//! bespoke effect). The dispatch from `EffectId` → constructor goes through
//! [`super::custom_effect::make_custom`].

pub mod aura;
