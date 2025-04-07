#![allow(dead_code)]

use constants::*;
use prelude::*;

#[repr(C)]
pub struct Sha3Context(usize);
#[repr(C)]
pub struct Secp256k1Context(usize);
#[repr(C)]
pub struct Secp256k1AggsigContext(usize);
#[repr(C)]
pub struct CsprngCtx(usize);
#[repr(C)]
pub struct PublicKeyUncompressed(pub [u8; 64]);
#[repr(C)]
pub struct SecretKey(pub [u8; 32]);
#[repr(C)]
pub struct AggSigPartialSignature(pub [u8; 32]);
#[repr(C)]
pub struct Signature(pub [u8; 64]);
#[repr(C)]
#[derive(Clone)]
pub struct SignatureScalar(pub [u8; 32]);
#[repr(C)]
pub struct CommitmentUncompressed(pub [u8; 64]);
#[repr(C)]
pub struct ScratchSpace(usize);
#[repr(C)]
pub struct BulletproofGenerators(usize);
#[repr(C)]
pub struct Commitment(pub [u8; 33]);
#[repr(C)]
pub struct PublicKey(pub [u8; 33]);

static mut SHARED_BULLETGENERATORS: Option<*mut BulletproofGenerators> = None;

pub unsafe fn shared_generators(ctx: *mut Secp256k1Context) -> *mut BulletproofGenerators {
	use ffi::*;
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

#[cfg(test)]
mod test {
	use super::*;
	use core::panic;
	use core::ptr;
	use ffi::*;

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
				skeys.as_ptr() as *const SecretKey,
			);
			// create second pubkey frommm skey
			secp256k1_ec_pubkey_create(
				ctx,
				(&pkeys as *const u8).add(64) as *mut PublicKeyUncompressed,
				(&skeys as *const u8).add(32) as *const SecretKey,
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

			let mut blind1 = SecretKey([0u8; 32]);
			let mut blind2 = SecretKey([0u8; 32]);
			cpsrng_rand_bytes(r, blind1.0.as_mut_ptr(), 32);
			cpsrng_rand_bytes(r, blind2.0.as_mut_ptr(), 32);

			let mut c1 = CommitmentUncompressed([0u8; 64]);
			let mut c2 = CommitmentUncompressed([0u8; 64]);
			assert_eq!(
				secp256k1_pedersen_commit(
					ctx,
					&mut c1 as *mut CommitmentUncompressed,
					&blind1 as *const SecretKey,
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
					&blind2 as *const SecretKey,
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
			let mut blind = SecretKey([0u8; 32]);
			cpsrng_rand_bytes(r, blind.0.as_mut_ptr(), 32);
			let value = 1000u64;
			let mut commit = CommitmentUncompressed([0u8; 64]);
			assert_eq!(
				secp256k1_pedersen_commit(
					ctx,
					&mut commit as *mut CommitmentUncompressed,
					&blind as *const SecretKey,
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
			let blinds = [&blind as *const SecretKey];
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

			let mut blind1 = SecretKey([0u8; 32]);
			let mut blind2 = SecretKey([0u8; 32]);
			let mut blind3 = SecretKey([0u8; 32]);
			let mut blind4 = SecretKey([0u8; 32]);
			cpsrng_rand_bytes(r, blind1.0.as_mut_ptr(), 32);
			cpsrng_rand_bytes(r, blind2.0.as_mut_ptr(), 32);
			cpsrng_rand_bytes(r, blind3.0.as_mut_ptr(), 32);

			// Compute blind4 = blind1 + blind2 - blind3 using tweak add and blind sum
			let mut sum_in = SecretKey(blind1.0); // Copy blind1
			assert_eq!(
				secp256k1_ec_privkey_tweak_add(ctx, &mut sum_in, &blind2),
				1,
				"Tweak add failed for input blinds"
			);
			let blinds = [&sum_in as *const SecretKey, &blind3 as *const SecretKey];
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
					&blind1 as *const SecretKey,
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
					&blind2 as *const SecretKey,
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
					&blind3 as *const SecretKey,
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
					&blind4 as *const SecretKey,
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

	#[test]
	fn test_mem() {
		unsafe {
			let start = getalloccount();
			let x = alloc(100);
			let y = resize(x, 200);
			assert!(start != getalloccount());
			release(y);
			assert_eq!(start, getalloccount());
		}
	}
}
