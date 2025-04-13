#[repr(C)]
pub struct MDB_env(i32); // Opaque
#[repr(C)]
pub struct MDB_txn(i32); // Opaque
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MDB_dbi(pub u32);

#[repr(C)]
pub struct MDB_val {
	pub mv_size: usize,
	pub mv_data: *mut u8,
}
