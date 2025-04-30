#[macro_use]
pub mod macros;

mod constants;
mod errors;
mod ffi;
mod node;

pub mod channel;
pub mod hashtable;
pub mod lock;
pub mod murmur;
pub mod rbtree;
pub mod thread;

use util::hashtable::Murmur3Hasher;
pub type Hashtable<K, V> = crate::util::hashtable::Hashtable<K, V, Murmur3Hasher>;
