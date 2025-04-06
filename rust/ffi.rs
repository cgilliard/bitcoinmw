#![allow(dead_code)]

use types::*;

extern "C" {
	// memory allocation
	pub fn alloc(bytes: usize) -> *const u8;
	pub fn release(ptr: *const u8);
	pub fn resize(ptr: *const u8, bytes: usize) -> *const u8;

	// sys
	pub fn write(fd: i32, buf: *const u8, len: usize) -> i32;
	pub fn ptr_add(p: *mut u8, v: i64);
	pub fn exit(code: i32);
	pub fn getalloccount() -> usize;

	// misc
	pub fn sleep_millis(millis: u64) -> i32;
	pub fn f64_to_str(d: f64, buf: *mut u8, capacity: u64) -> i32;

	// atomic
	pub fn atomic_store_u64(ptr: *mut u64, value: u64);
	pub fn atomic_load_u64(ptr: *const u64) -> u64;
	pub fn atomic_fetch_add_u64(ptr: *mut u64, value: u64) -> u64;
	pub fn atomic_fetch_sub_u64(ptr: *mut u64, value: u64) -> u64;
	pub fn cas_release(ptr: *mut u64, expect: *const u64, desired: u64) -> bool;

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
	pub fn secp256k1_ec_seckey_verify(cx: *const Secp256k1Context, sk: *const SecretKey) -> i32;
	pub fn secp256k1_ec_pubkey_create(
		cx: *const Secp256k1Context,
		pk: *mut PublicKeyUncompressed,
		sk: *const SecretKey,
	) -> i32;

	// Parse a 33-byte commitment into 64 byte internal commitment object
	pub fn secp256k1_pedersen_commitment_parse(
		cx: *const Secp256k1Context,
		commit: *mut u8,
		input: *const u8,
	) -> i32;

	// Serialize a 64-byte commit object into a 33 byte serialized byte sequence
	pub fn secp256k1_pedersen_commitment_serialize(
		cx: *const Secp256k1Context,
		output: *mut u8,
		commit: *const u8,
	) -> i32;

	pub fn secp256k1_ec_pubkey_serialize(
		cx: *const Secp256k1Context,
		output: *mut u8,
		outputlen: *const usize,
		pubkey: *const PublicKeyUncompressed,
		flags: u32,
	) -> i32;

	pub fn secp256k1_ec_pubkey_parse(
		cx: *const Secp256k1Context,
		pk: *mut PublicKeyUncompressed,
		input: *const u8,
		intputlen: usize,
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
		nsigs: usize,
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
		blind: *const SecretKey,
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
		seckey: *mut SecretKey,
		tweak: *const SecretKey,
	) -> i32;

	// Pedersen blind sum for combining blinding factors
	pub fn secp256k1_pedersen_blind_sum(
		cx: *const Secp256k1Context,
		blind_out: *mut SecretKey,
		blinds: *const *const SecretKey,
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
		blind: *const *const SecretKey,
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
