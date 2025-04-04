#![allow(dead_code)]

#[repr(C)]
pub struct Secp256k1Context(usize);
#[repr(C)]
pub struct Secp256k1AggsigContext(usize);
#[repr(C)]
pub struct CsprngCtx(usize);
#[repr(C)]
pub struct PublicKey([u8; 64]); // pubkey
#[repr(C)]
pub struct SecretKey([u8; 32]); // Secret key
#[repr(C)]
pub struct AggSigPartialSignature([u8; 32]); // Partial signature
#[repr(C)]
pub struct Signature([u8; 64]); // Final signature
#[repr(C)]
pub struct Commitment([u8; 64]);

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

extern "C" {
	// secp256k1
	pub fn secp256k1_context_create(flags: u32) -> *mut Secp256k1Context;
	pub fn secp256k1_context_destroy(ctx: *mut Secp256k1Context);

	pub fn secp256k1_aggsig_context_create(
		cx: *const Secp256k1Context,
		pks: *const PublicKey,
		n_pks: usize,
		seed32: *const u8,
	) -> *mut Secp256k1AggsigContext;
	pub fn secp256k1_aggsig_context_destroy(aggctx: *mut Secp256k1AggsigContext);
	pub fn secp256k1_ec_seckey_verify(cx: *const Secp256k1Context, sk: *const SecretKey) -> i32;
	pub fn secp256k1_ec_pubkey_create(
		cx: *const Secp256k1Context,
		pk: *mut PublicKey,
		sk: *const SecretKey,
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
		seckey32: *const SecretKey,
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
		pks: *const PublicKey,
		n_pubkeys: usize,
	) -> i32;

	// Pedersen commitments
	pub fn secp256k1_pedersen_commit(
		cx: *const Secp256k1Context,
		commit: *mut Commitment,
		blind: *const SecretKey,
		value: u64,
		value_gen: *const PublicKey,
		blind_gen: *const PublicKey,
	) -> i32;

	pub fn secp256k1_pedersen_commit_sum(
		cx: *const Secp256k1Context,
		commit_out: *mut Commitment,
		commits: *const *const Commitment,
		pcnt: usize,
		ncommits: *const *const Commitment,
		ncnt: usize,
	) -> i32;

	pub fn secp256k1_pedersen_verify_tally(
		cx: *const Secp256k1Context,
		commits: *const *const Commitment,
		n_commits: usize,
		neg_commits: *const *const Commitment,
		n_neg_commits: usize,
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
				pkeys.as_mut_ptr() as *mut PublicKey,
				skeys.as_ptr() as *const SecretKey,
			);
			// create second pubkey frommm skey
			secp256k1_ec_pubkey_create(
				ctx,
				(&pkeys as *const u8).add(64) as *mut PublicKey,
				(&skeys as *const u8).add(32) as *const SecretKey,
			);

			// create the aggsig context
			let aggctx = secp256k1_aggsig_context_create(
				ctx,
				&pkeys[0] as *const u8 as *const PublicKey,
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
					skeys.as_ptr() as *const SecretKey,
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
					(&skeys as *const u8).add(32) as *const SecretKey,
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
				pkeys.as_ptr() as *const PublicKey,
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

			let mut blind1 = SecretKey([0u8; 32]);
			let mut blind2 = SecretKey([0u8; 32]);
			cpsrng_rand_bytes(r, blind1.0.as_mut_ptr(), 32);
			cpsrng_rand_bytes(r, blind2.0.as_mut_ptr(), 32);

			let mut gen_g = PublicKey([0u8; 64]);
			let mut gen_h = PublicKey([0u8; 64]);
			secp256k1_ec_pubkey_create(
				ctx,
				&mut gen_g as *mut PublicKey,
				blind1.0.as_ptr() as *const SecretKey,
			);
			secp256k1_ec_pubkey_create(
				ctx,
				&mut gen_h as *mut PublicKey,
				blind2.0.as_ptr() as *const SecretKey,
			);

			let mut c1 = Commitment([0u8; 64]);
			let mut c2 = Commitment([0u8; 64]);
			assert_eq!(
				secp256k1_pedersen_commit(
					ctx,
					&mut c1 as *mut Commitment,
					&blind1 as *const SecretKey,
					1000,
					&gen_h as *const PublicKey,
					&gen_g as *const PublicKey
				),
				1
			);
			assert_eq!(
				secp256k1_pedersen_commit(
					ctx,
					&mut c2 as *mut Commitment,
					&blind2 as *const SecretKey,
					2000,
					&gen_h as *const PublicKey,
					&gen_g as *const PublicKey
				),
				1
			);

			let mut sum = Commitment([0u8; 64]);
			let commits = [&c1 as *const Commitment];
			let ncommits = [&c2 as *const Commitment];
			assert_eq!(
				secp256k1_pedersen_commit_sum(
					ctx,
					&mut sum as *mut Commitment,
					commits.as_ptr(),
					1,
					ncommits.as_ptr(),
					1
				),
				1
			);

			secp256k1_context_destroy(ctx);
			cpsrng_context_destroy(r);
		}
	}
}
