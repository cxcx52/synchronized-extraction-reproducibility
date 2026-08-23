pub const RING_DEGREE: usize = 128;
pub static DEGREE: usize = RING_DEGREE;
#[cfg(not(feature = "quartic-q"))]
pub const EXTENSION_DEGREE: usize = 2;
#[cfg(feature = "quartic-q")]
pub const EXTENSION_DEGREE: usize = 4;
pub const NUM_SLOTS: usize = RING_DEGREE / EXTENSION_DEGREE;
/// Historical compatibility name.  It denotes the CRT slot count even when
/// the configured extension degree is four.
pub static HALF_DEGREE: usize = NUM_SLOTS;
#[cfg(not(feature = "quartic-q"))]
pub static MOD_Q: u64 = 1125899906839937;
#[cfg(feature = "quartic-q")]
pub static MOD_Q: u64 = 926510094425921;

pub static NOF_BATCHES: usize = 2;
