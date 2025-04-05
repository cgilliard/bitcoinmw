#![allow(dead_code)]

use core::option::Option;
use core::option::Option::{None, Some};

pub const MAX_GENERATORS: usize = 256;

#[repr(C)]
pub struct Secp256k1Context(usize);
#[repr(C)]
pub struct Secp256k1AggsigContext(usize);
#[repr(C)]
pub struct CsprngCtx(usize);
#[repr(C)]
pub struct PublicKeyUncompressed([u8; 64]); // pubkey
#[repr(C)]
pub struct SecretKeyImpl([u8; 32]); // Secret key
#[repr(C)]
pub struct AggSigPartialSignature([u8; 32]); // Partial signature
#[repr(C)]
pub struct Signature([u8; 64]); // Final signature
#[repr(C)]
pub struct CommitmentUncompressed([u8; 64]);
#[repr(C)]
pub struct ScratchSpace(usize);
#[repr(C)]
pub struct BulletproofGenerators(usize);

pub const GENERATOR_G: PublicKeyUncompressed = PublicKeyUncompressed([
	0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce, 0x87, 0x0b, 0x07,
	0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81, 0x5b, 0x16, 0xf8, 0x17, 0x98,
	0x48, 0x3a, 0xda, 0x77, 0x26, 0xa3, 0xc4, 0x65, 0x5d, 0xa4, 0xfb, 0xfc, 0x0e, 0x11, 0x08, 0xa8,
	0xfd, 0x17, 0xb4, 0x48, 0xa6, 0x85, 0x54, 0x19, 0x9c, 0x47, 0xd0, 0x8f, 0xfb, 0x10, 0xd4, 0xb8,
]);

pub const GENERATOR_H: PublicKeyUncompressed = PublicKeyUncompressed([
	0x50, 0x92, 0x9b, 0x74, 0xc1, 0xa0, 0x49, 0x54, 0xb7, 0x8b, 0x4b, 0x60, 0x35, 0xe9, 0x7a, 0x5e,
	0x07, 0x8a, 0x5a, 0x0f, 0x28, 0xec, 0x96, 0xd5, 0x47, 0xbf, 0xee, 0x9a, 0xce, 0x80, 0x3a, 0xc0,
	0x31, 0xd3, 0xc6, 0x86, 0x39, 0x73, 0x92, 0x6e, 0x04, 0x9e, 0x63, 0x7c, 0xb1, 0xb5, 0xf4, 0x0a,
	0x36, 0xda, 0xc2, 0x8a, 0xf1, 0x76, 0x69, 0x68, 0xc3, 0x0c, 0x23, 0x13, 0xf3, 0xa3, 0x89, 0x04,
]);

/// Flag for context to enable no precomputation
pub const SECP256K1_START_NONE: u32 = (1 << 0) | 0;
/// Flag for context to enable verification precomputation
pub const SECP256K1_START_VERIFY: u32 = (1 << 0) | (1 << 8);
/// Flag for context to enable signing precomputation
pub const SECP256K1_START_SIGN: u32 = (1 << 0) | (1 << 9);
/// Flag for keys to indicate uncompressed serialization format
pub const SECP256K1_SER_UNCOMPRESSED: u32 = (1 << 1) | 0;
/// Flag for keys to indicate compressed serialization format
pub const SECP256K1_SER_COMPRESSED: u32 = (1 << 1) | (1 << 8);

static mut SHARED_BULLETGENERATORS: Option<*mut BulletproofGenerators> = None;

unsafe fn shared_generators(ctx: *mut Secp256k1Context) -> *mut BulletproofGenerators {
	match SHARED_BULLETGENERATORS {
		Some(gens) => gens,
		None => {
			let gens = secp256k1_bulletproof_generators_create(
				ctx,
				&GENERATOR_G as *const PublicKeyUncompressed,
				MAX_GENERATORS,
			);
			SHARED_BULLETGENERATORS = Some(gens);
			gens
		}
	}
}

extern "C" {
	// secp256k1
	pub fn secp256k1_context_create(flags: u32) -> *mut Secp256k1Context;
	pub fn secp256k1_context_destroy(ctx: *mut Secp256k1Context);

	pub fn secp256k1_aggsig_context_create(
		cx: *const Secp256k1Context,
		pks: *const PublicKeyUncompressed,
		n_pks: usize,
		seed32: *const u8,
	) -> *mut Secp256k1AggsigContext;
	pub fn secp256k1_aggsig_context_destroy(aggctx: *mut Secp256k1AggsigContext);
	pub fn secp256k1_ec_seckey_verify(cx: *const Secp256k1Context, sk: *const SecretKeyImpl) -> i32;
	pub fn secp256k1_ec_pubkey_create(
		cx: *const Secp256k1Context,
		pk: *mut PublicKeyUncompressed,
		sk: *const SecretKeyImpl,
	) -> i32;

	pub fn secp256k1_aggsig_generate_nonce(
		cx: *const Secp256k1Context,
		aggctx: *mut Secp256k1AggsigContext,
		index: usize,
	) -> i32;

	pub fn secp256k1_aggsig_partial_sign(
		cx: *const Secp256k1Context,
		aggctx: *mut Secp256k1AggsigContext,
		sig: *mut AggSigPartialSignature,
		msghash32: *const u8,
		seckey32: *const SecretKeyImpl,
		index: usize,
	) -> i32;

	pub fn secp256k1_aggsig_combine_signatures(
		cx: *const Secp256k1Context,
		aggctx: *mut Secp256k1AggsigContext,
		sig64: *mut Signature,
		partial: *const AggSigPartialSignature,
		index: usize,
	) -> i32;

	pub fn secp256k1_aggsig_build_scratch_and_verify(
		cx: *const Secp256k1Context,
		sig64: *const Signature,
		msg32: *const u8,
		pks: *const PublicKeyUncompressed,
		n_pubkeys: usize,
	) -> i32;

	pub fn secp256k1_scratch_space_create(
		ctx: *const Secp256k1Context,
		max_size: usize,
	) -> *mut ScratchSpace;
	pub fn secp256k1_scratch_space_destroy(scratch: *mut ScratchSpace);

	// Pedersen commitments
	pub fn secp256k1_pedersen_commit(
		cx: *const Secp256k1Context,
		commit: *mut CommitmentUncompressed,
		blind: *const SecretKeyImpl,
		value: u64,
		value_gen: *const PublicKeyUncompressed,
		blind_gen: *const PublicKeyUncompressed,
	) -> i32;

	pub fn secp256k1_pedersen_commit_sum(
		cx: *const Secp256k1Context,
		commit_out: *mut CommitmentUncompressed,
		commits: *const *const CommitmentUncompressed,
		pcnt: usize,
		ncommits: *const *const CommitmentUncompressed,
		ncnt: usize,
	) -> i32;

	pub fn secp256k1_pedersen_verify_tally(
		cx: *const Secp256k1Context,
		commits: *const *const CommitmentUncompressed,
		n_commits: usize,
		neg_commits: *const *const CommitmentUncompressed,
		n_neg_commits: usize,
	) -> i32;

	// Tweak operations for scalar arithmetic
	pub fn secp256k1_ec_privkey_tweak_add(
		cx: *const Secp256k1Context,
		seckey: *mut SecretKeyImpl,
		tweak: *const SecretKeyImpl,
	) -> i32;

	// Pedersen blind sum for combining blinding factors
	pub fn secp256k1_pedersen_blind_sum(
		cx: *const Secp256k1Context,
		blind_out: *mut SecretKeyImpl,
		blinds: *const *const SecretKeyImpl,
		nblinds: usize,
		npositive: usize,
	) -> i32;

	// Range proof
	pub fn secp256k1_bulletproof_generators_create(
		ctx: *const Secp256k1Context,
		blinding_gen: *const PublicKeyUncompressed,
		n: usize,
	) -> *mut BulletproofGenerators;
	pub fn secp256k1_bulletproof_generators_destroy(
		ctx: *const Secp256k1Context,
		gens: *mut BulletproofGenerators,
	);

	pub fn secp256k1_bulletproof_rangeproof_prove(
		ctx: *const Secp256k1Context,
		scratch: *mut ScratchSpace,
		gens: *const BulletproofGenerators,
		proof: *mut u8,
		plen: *mut usize,
		tau_x: *mut [u8; 32],
		t_one: *mut PublicKeyUncompressed,
		t_two: *mut PublicKeyUncompressed,
		value: *const u64,
		min_value: *const u64,
		blind: *const *const SecretKeyImpl,
		commits: *const *const CommitmentUncompressed,
		n_commits: usize,
		value_gen: *const PublicKeyUncompressed,
		nbits: usize,
		nonce: *const [u8; 32],
		private_nonce: *const [u8; 32],
		extra_commit: *const u8,
		extra_commit_len: usize,
		message: *const [u8; 20],
	) -> i32;

	pub fn secp256k1_bulletproof_rangeproof_verify(
		ctx: *const Secp256k1Context,
		scratch: *mut ScratchSpace,
		gens: *const BulletproofGenerators,
		proof: *const u8,
		plen: usize,
		min_value: *const u64,
		commit: *const CommitmentUncompressed,
		n_commits: usize,
		nbits: usize,
		value_gen: *const PublicKeyUncompressed,
		extra_commit: *const u8,
		extra_commit_len: usize,
	) -> i32;

	// cpsrng
	pub fn cpsrng_reseed();
	pub fn cpsrng_context_create() -> *mut CsprngCtx;
	pub fn cpsrng_context_destroy(ctx: *mut CsprngCtx);
	pub fn cpsrng_rand_bytes(ctx: *mut CsprngCtx, v: *mut u8, size: usize);

	// Only in tests
	pub fn cpsrng_test_seed(ctx: *mut CsprngCtx, iv16: *const u8, key32: *const u8);
}

#[cfg(test)]
mod test {
	use super::*;
	use core::panic;
	use core::ptr;

	pub const MAX_WIDTH: usize = 1 << 20; // 1,048,576
	pub const SCRATCH_SPACE_SIZE: usize = 256 * MAX_WIDTH; // ~256 MB

	#[test]
	fn test_rand() {
		unsafe {
			let r = cpsrng_context_create();
			let iv = [0u8; 16];
			let key = [1u8; 32];
			cpsrng_test_seed(r, iv.as_ptr(), key.as_ptr());
			let mut v = [0u8; 1];
			cpsrng_rand_bytes(r, v.as_mut_ptr(), 1);
			assert_eq!(v[0], 174);
			cpsrng_context_destroy(r);
		}
	}

	#[test]
	fn test_aggsig() {
		unsafe {
			// init cpsrng with a test seed
			let r = cpsrng_context_create();
			let iv = [0u8; 16];
			let key = [2u8; 32];
			cpsrng_test_seed(r, iv.as_ptr(), key.as_ptr());

			// create secp256k1 context
			let ctx = secp256k1_context_create(SECP256K1_START_SIGN | SECP256K1_START_VERIFY);

			// create space for two pubkeys
			let mut pkeys = [9u8; 128];
			// create space for two secret keys
			let mut skeys = [0u8; 64];
			// seed (use 4u8)
			let seed = [4u8; 32];

			// randomize the skeys
			cpsrng_rand_bytes(r, skeys.as_mut_ptr(), 128);
			// create first pubkey from skey
			secp256k1_ec_pubkey_create(
				ctx,
				pkeys.as_mut_ptr() as *mut PublicKeyUncompressed,
				skeys.as_ptr() as *const SecretKeyImpl,
			);
			// create second pubkey frommm skey
			secp256k1_ec_pubkey_create(
				ctx,
				(&pkeys as *const u8).add(64) as *mut PublicKeyUncompressed,
				(&skeys as *const u8).add(32) as *const SecretKeyImpl,
			);

			// create the aggsig context
			let aggctx = secp256k1_aggsig_context_create(
				ctx,
				&pkeys[0] as *const u8 as *const PublicKeyUncompressed,
				2,
				seed.as_ptr(),
			);

			// generate two nonces
			for i in 0..2 {
				assert_eq!(secp256k1_aggsig_generate_nonce(ctx, aggctx, i), 1);
			}

			// allocate for two partial signatures
			let mut partial_sigs = [0u8; 64];
			// use a 32 byte message
			let msg32 = [9u8; 32];

			// sign first partial
			assert_eq!(
				secp256k1_aggsig_partial_sign(
					ctx,
					aggctx,
					&mut partial_sigs as *mut u8 as *mut AggSigPartialSignature,
					msg32.as_ptr(),
					skeys.as_ptr() as *const SecretKeyImpl,
					0,
				),
				1
			);

			// sign second partial
			assert_eq!(
				secp256k1_aggsig_partial_sign(
					ctx,
					aggctx,
					(&mut partial_sigs as *mut u8).add(32) as *mut AggSigPartialSignature,
					msg32.as_ptr(),
					(&skeys as *const u8).add(32) as *const SecretKeyImpl,
					1,
				),
				1
			);

			// combine into final_sig
			let mut final_sig = Signature([0u8; 64]);
			assert_eq!(
				secp256k1_aggsig_combine_signatures(
					ctx,
					aggctx,
					&mut final_sig as *mut Signature,
					&mut partial_sigs as *mut u8 as *mut AggSigPartialSignature,
					2,
				),
				1
			);

			// verify final signature
			let result = secp256k1_aggsig_build_scratch_and_verify(
				ctx,
				&final_sig as *const Signature,
				msg32.as_ptr(),
				pkeys.as_ptr() as *const PublicKeyUncompressed,
				2,
			);
			assert_eq!(result, 1, "Verification failed: {}", result);

			// destroy
			secp256k1_aggsig_context_destroy(aggctx);
			secp256k1_context_destroy(ctx);
			cpsrng_context_destroy(r);
		}
	}

	#[test]
	fn test_pedersen_commit() {
		unsafe {
			let ctx = secp256k1_context_create(SECP256K1_START_SIGN | SECP256K1_START_VERIFY);
			let r = cpsrng_context_create();
			let iv = [0u8; 16];
			let key = [2u8; 32];
			cpsrng_test_seed(r, iv.as_ptr(), key.as_ptr());

			let mut blind1 = SecretKeyImpl([0u8; 32]);
			let mut blind2 = SecretKeyImpl([0u8; 32]);
			cpsrng_rand_bytes(r, blind1.0.as_mut_ptr(), 32);
			cpsrng_rand_bytes(r, blind2.0.as_mut_ptr(), 32);

			let mut c1 = CommitmentUncompressed([0u8; 64]);
			let mut c2 = CommitmentUncompressed([0u8; 64]);
			assert_eq!(
				secp256k1_pedersen_commit(
					ctx,
					&mut c1 as *mut CommitmentUncompressed,
					&blind1 as *const SecretKeyImpl,
					1000,
					&GENERATOR_H as *const PublicKeyUncompressed,
					&GENERATOR_G as *const PublicKeyUncompressed
				),
				1
			);
			assert_eq!(
				secp256k1_pedersen_commit(
					ctx,
					&mut c2 as *mut CommitmentUncompressed,
					&blind2 as *const SecretKeyImpl,
					2000,
					&GENERATOR_H as *const PublicKeyUncompressed,
					&GENERATOR_G as *const PublicKeyUncompressed
				),
				1
			);

			let mut sum = CommitmentUncompressed([0u8; 64]);
			let commits = [&c1 as *const CommitmentUncompressed];
			let ncommits = [&c2 as *const CommitmentUncompressed];
			assert_eq!(
				secp256k1_pedersen_commit_sum(
					ctx,
					&mut sum as *mut CommitmentUncompressed,
					commits.as_ptr(),
					1,
					ncommits.as_ptr(),
					1
				),
				1
			);

			// Verify: c1 = c2 + sum (i.e., c1 - c2 - sum = 0)
			let positive = [&c1 as *const CommitmentUncompressed];
			let negative = [
				&c2 as *const CommitmentUncompressed,
				&sum as *const CommitmentUncompressed,
			];
			assert_eq!(
				secp256k1_pedersen_verify_tally(ctx, positive.as_ptr(), 1, negative.as_ptr(), 2),
				1
			);

			secp256k1_context_destroy(ctx);
			cpsrng_context_destroy(r);
		}
	}

	#[test]
	fn test_bulletproof() {
		unsafe {
			let ctx = secp256k1_context_create(SECP256K1_START_SIGN | SECP256K1_START_VERIFY);
			assert!(!ctx.is_null(), "Context creation failed");

			let r = cpsrng_context_create();
			let iv = [0u8; 16];
			let key = [2u8; 32];
			cpsrng_test_seed(r, iv.as_ptr(), key.as_ptr());

			// Create a commitment
			let mut blind = SecretKeyImpl([0u8; 32]);
			cpsrng_rand_bytes(r, blind.0.as_mut_ptr(), 32);
			let value = 1000u64;
			let mut commit = CommitmentUncompressed([0u8; 64]);
			assert_eq!(
				secp256k1_pedersen_commit(
					ctx,
					&mut commit as *mut CommitmentUncompressed,
					&blind as *const SecretKeyImpl,
					value,
					&GENERATOR_H as *const PublicKeyUncompressed,
					&GENERATOR_G as *const PublicKeyUncompressed
				),
				1
			);

			// Scratch space
			let scratch = secp256k1_scratch_space_create(ctx, SCRATCH_SPACE_SIZE);
			assert!(!scratch.is_null(), "Scratch space creation failed");

			// Shared generators (Grin’s approach)
			let gens = shared_generators(ctx);
			assert!(!gens.is_null(), "Shared generators failed");

			// Prove range
			let mut proof = [0u8; 1024];
			let mut proof_len = 1024usize;
			let nonce = [3u8; 32];
			let blinds = [&blind as *const SecretKeyImpl];
			let result = secp256k1_bulletproof_rangeproof_prove(
				ctx,
				scratch,
				gens,
				proof.as_mut_ptr(),
				&mut proof_len,
				ptr::null_mut(),
				ptr::null_mut(),
				ptr::null_mut(),
				&value as *const u64,
				ptr::null(),
				blinds.as_ptr(),
				ptr::null_mut(),
				1,
				&GENERATOR_H as *const PublicKeyUncompressed,
				64,
				&nonce,
				ptr::null(),
				ptr::null(),
				0,
				ptr::null(),
			);
			assert_eq!(result, 1, "Bulletproof prove failed: {}", result);

			// Verify range
			assert_eq!(
				secp256k1_bulletproof_rangeproof_verify(
					ctx,
					scratch,
					gens,
					proof.as_ptr(),
					proof_len,
					ptr::null(),
					&commit as *const CommitmentUncompressed,
					1,
					64,
					&GENERATOR_H as *const PublicKeyUncompressed,
					ptr::null(),
					0
				),
				1
			);

			secp256k1_scratch_space_destroy(scratch);
			secp256k1_context_destroy(ctx);
			cpsrng_context_destroy(r);
		}
	}

	#[test]
	fn test_transaction() {
		unsafe {
			let ctx = secp256k1_context_create(SECP256K1_START_SIGN | SECP256K1_START_VERIFY);
			let r = cpsrng_context_create();
			let iv = [0u8; 16];
			let key = [2u8; 32];
			cpsrng_test_seed(r, iv.as_ptr(), key.as_ptr());

			let mut blind1 = SecretKeyImpl([0u8; 32]);
			let mut blind2 = SecretKeyImpl([0u8; 32]);
			let mut blind3 = SecretKeyImpl([0u8; 32]);
			let mut blind4 = SecretKeyImpl([0u8; 32]);
			cpsrng_rand_bytes(r, blind1.0.as_mut_ptr(), 32);
			cpsrng_rand_bytes(r, blind2.0.as_mut_ptr(), 32);
			cpsrng_rand_bytes(r, blind3.0.as_mut_ptr(), 32);

			// Compute blind4 = blind1 + blind2 - blind3 using tweak add and blind sum
			let mut sum_in = SecretKeyImpl(blind1.0); // Copy blind1
			assert_eq!(
				secp256k1_ec_privkey_tweak_add(ctx, &mut sum_in, &blind2),
				1,
				"Tweak add failed for input blinds"
			);
			let blinds = [&sum_in as *const SecretKeyImpl, &blind3 as *const SecretKeyImpl];
			assert_eq!(
				secp256k1_pedersen_blind_sum(ctx, &mut blind4, blinds.as_ptr(), 2, 1), // 1 positive (sum_in), 1 negative (blind3)
				1,
				"Blind sum failed for output blind"
			);

			let mut input1 = CommitmentUncompressed([0u8; 64]);
			let mut input2 = CommitmentUncompressed([0u8; 64]);
			let mut output1 = CommitmentUncompressed([0u8; 64]);
			let mut output2 = CommitmentUncompressed([0u8; 64]);
			assert_eq!(
				secp256k1_pedersen_commit(
					ctx,
					&mut input1 as *mut CommitmentUncompressed,
					&blind1 as *const SecretKeyImpl,
					1000,
					&GENERATOR_H as *const PublicKeyUncompressed,
					&GENERATOR_G as *const PublicKeyUncompressed
				),
				1
			);
			assert_eq!(
				secp256k1_pedersen_commit(
					ctx,
					&mut input2 as *mut CommitmentUncompressed,
					&blind2 as *const SecretKeyImpl,
					3000,
					&GENERATOR_H as *const PublicKeyUncompressed,
					&GENERATOR_G as *const PublicKeyUncompressed
				),
				1
			);
			assert_eq!(
				secp256k1_pedersen_commit(
					ctx,
					&mut output1 as *mut CommitmentUncompressed,
					&blind3 as *const SecretKeyImpl,
					2000,
					&GENERATOR_H as *const PublicKeyUncompressed,
					&GENERATOR_G as *const PublicKeyUncompressed
				),
				1
			);
			assert_eq!(
				secp256k1_pedersen_commit(
					ctx,
					&mut output2 as *mut CommitmentUncompressed,
					&blind4 as *const SecretKeyImpl,
					2000,
					&GENERATOR_H as *const PublicKeyUncompressed,
					&GENERATOR_G as *const PublicKeyUncompressed
				),
				1
			);

			let positive = [
				&input1 as *const CommitmentUncompressed,
				&input2 as *const CommitmentUncompressed,
			];
			let negative = [
				&output1 as *const CommitmentUncompressed,
				&output2 as *const CommitmentUncompressed,
			];
			assert_eq!(
				secp256k1_pedersen_verify_tally(ctx, positive.as_ptr(), 2, negative.as_ptr(), 2),
				1
			);

			secp256k1_context_destroy(ctx);
			cpsrng_context_destroy(r);
		}
	}
}
