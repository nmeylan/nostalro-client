pub mod debug;

#[cfg(feature = "puffin")]
mod server;

#[cfg(feature = "puffin")]
pub use server::Profiler;

#[cfg(not(feature = "puffin"))]
#[derive(Default)]
pub struct Profiler;

#[cfg(not(feature = "puffin"))]
impl Profiler {
    pub fn start(&mut self) {}
    pub fn new_frame(&self) {}
}

pub mod reexports {
    #[cfg(feature = "puffin")]
    pub use puffin;
}

#[cfg(feature = "profiling")]
#[macro_export]
macro_rules! __profile_dispatch {
    ($m:ident $($arg: tt)*) => {
        $crate::reexports::puffin::$m!($($arg)*);
    };
}

#[cfg(not(feature = "profiling"))]
#[macro_export]
macro_rules! __profile_dispatch {
    ($m:ident $($arg: tt)*) => {};
}

#[macro_export]
macro_rules! profile_function {
    ($($arg: tt)*) => {
        $crate::__profile_dispatch!(profile_function $($arg)*);
    };
}

#[macro_export]
macro_rules! profile_function_if {
    ($($arg: tt)*) => {
        $crate::__profile_dispatch!(profile_function_if $($arg)*);
    };
}

#[macro_export]
macro_rules! profile_scope {
    ($($arg: tt)*) => {
        $crate::__profile_dispatch!(profile_scope $($arg)*);
    };
}

#[macro_export]
macro_rules! profile_scope_if {
    ($($arg: tt)*) => {
        $crate::__profile_dispatch!(profile_scope_if $($arg)*);
    };
}
