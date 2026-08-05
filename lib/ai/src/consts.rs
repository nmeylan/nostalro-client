//! Enum vocabularies shared by config, tactics, and the engine. Each enum
//! serializes as its reference integer so the JSON stays aligned with the
//! reference AI's published documentation; unknown integers self-heal to the
//! declared default variant.

macro_rules! int_enum {
    ($(#[$m:meta])* $name:ident { $($variant:ident = $val:literal),+ $(,)? } default $def:ident) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        #[serde(from = "i32", into = "i32")]
        pub enum $name { $($variant),+ }

        impl From<i32> for $name {
            fn from(v: i32) -> Self {
                match v { $($val => $name::$variant,)+ _ => $name::$def }
            }
        }
        impl From<$name> for i32 {
            fn from(v: $name) -> i32 {
                match v { $($name::$variant => $val,)+ }
            }
        }
        impl Default for $name {
            fn default() -> Self { $name::$def }
        }
    };
}

int_enum!(UseSkillOnly { Attacking = 0, Chasing = -1, SkillOnly = 1 } default Chasing);
int_enum!(UseAutoPushback { Off = 0, SelfOnly = 1, All = 2 } default Off);
int_enum!(AutoMobMode { Disabled = 0, Aggressive = 1, All = 2 } default All);
int_enum!(AutoComboMode { Never = 0, Tactics = 1, Always = 2 } default Tactics);
int_enum!(UseIdleWalk {
    None = 0, Circle = 1, Cross = 2, Square = 3, Random = 4, RouteLinear = 5, RouteCircle = 6
} default None);
int_enum!(StickyStandby { Disabled = 0, Enabled = 1, EnabledRelog = 2 } default Enabled);
int_enum!(UseAutoHeal { Never = 0, Always = 1, Idle = 2, IdleLow = 3 } default Never);
int_enum!(BuffWhen {
    Chase = -1, IdleLow = -2, Never = 0, Idle = 1, Berserk = 2, Asap = 3
} default Never);
int_enum!(GroundBuffMode {
    Chase = -1, IdleLow = -2, Attack = 0, Idle = 1, Berserk = 2
} default Attack);

int_enum!(BasicTactic {
    TankMob = -2, Tank = -1, Ignore = 0, AttackLow = 2, AttackMed = 3, AttackHigh = 4,
    ReactLow = 5, ReactMed = 7, ReactHigh = 8, ReactSelf = 9,
    SnipeLow = 10, SnipeMed = 11, SnipeHigh = 12, AtkLowReactMed = 13,
    AttackLast = 14, AttackTop = 15
} default AttackMed);
int_enum!(KiteTactic { Never = 0, React = 1, Always = 2 } default React);
int_enum!(CastTactic { Passive = 0, React = 1 } default React);
int_enum!(PushbackTactic { Never = 0, SelfOnly = 1, Friend = 2 } default Never);
int_enum!(SkillClass {
    Both = -1, Old = 0, S = 1, Mob = 2, Combo1 = 3, Combo2 = 4, Minion = 5,
    Grapple = 6, Grapple1 = 7, Grapple2 = 8, MinOld = 9, MinS = 10
} default Both);
int_enum!(RescueTactic {
    Never = 0, Friend = 1, Retainer = 2, SelfOnly = 3, Owner = 4, All = 5
} default Never);
int_enum!(SnipeTactic { Disable = 0, Ok = 1 } default Ok);
int_enum!(KsTactic { Polite = -1, Never = 0, Always = 1 } default Never);
int_enum!(ChaseTactic { Always = 0, Never = 1, Clever = 2, Normal = -1 } default Normal);
int_enum!(FriendClass {
    Friend = 1, Retainer = 2, PkFriend = 3, Neutral = 10, Enemy = 11, Kos = 12, Ally = 13
} default Neutral);
