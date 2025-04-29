pub const fn wrapping_mul(a: u64, b: u64) -> u64 {
	// Split a and b into high and low 32-bit parts
	let a_low = (a & 0xFFFFFFFF) as u32;
	let a_high = ((a >> 32) & 0xFFFFFFFF) as u32;
	let b_low = (b & 0xFFFFFFFF) as u32;
	let b_high = ((b >> 32) & 0xFFFFFFFF) as u32;

	// Compute partial products
	let low_low = (a_low as u64) * (b_low as u64);
	let low_high = (a_low as u64) * (b_high as u64);
	let high_low = (a_high as u64) * (b_low as u64);

	// Combine for lower 64 bits
	let low = low_low;
	let mid = low_high + high_low + (low_low >> 32);

	// Final result: lower 32 bits of low + lower 32 bits of mid
	(low & 0xFFFFFFFF) | ((mid & 0xFFFFFFFF) << 32)
}

pub const fn simple_hash(s: &str, line: u32) -> u64 {
	let mut hash = 0xCBF29CE484222325_u64; // FNV-1a 64-bit offset basis
	const PRIME: u64 = 0x100000001B3; // FNV-1a 64-bit prime

	// Hash the string bytes
	let bytes = s.as_bytes();
	let mut i = 0;
	while i < bytes.len() {
		hash = hash ^ (bytes[i] as u64);
		hash = wrapping_mul(hash, PRIME);
		i += 1;
	}

	// Hash the line number (as 4 bytes, little-endian)
	hash = hash ^ ((line & 0xFF) as u64);
	hash = wrapping_mul(hash, PRIME);
	hash = hash ^ (((line >> 8) & 0xFF) as u64);
	hash = wrapping_mul(hash, PRIME);
	hash = hash ^ (((line >> 16) & 0xFF) as u64);
	hash = wrapping_mul(hash, PRIME);
	hash = hash ^ (((line >> 24) & 0xFF) as u64);
	hash = wrapping_mul(hash, PRIME);

	hash
}
