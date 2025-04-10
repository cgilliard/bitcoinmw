#![allow(dead_code)]
use crypto::keys::{Message, PublicKey, PublicKeyUncompressed, SecretKey, Signature};
use crypto::pedersen::{Commitment, CommitmentUncompressed};
use crypto::types::{
	BulletproofGenerators, CpsrngContext, ScratchSpace, Secp256k1Context, Sha3Context,
};

extern "C" {
	// sha3
	pub fn sha3_context_size() -> usize;
	pub fn sha3_Init(ctx: *const Sha3Context, bit_size: u32) -> i32;
	pub fn sha3_SetFlags(ctx: *const Sha3Context, flags: i32) -> i32;
	pub fn sha3_Update(ctx: *const Sha3Context, buf_in: *const u8, len: usize);
	pub fn sha3_Finalize(ctx: *const Sha3Context) -> *const u8;

	// cpsrng
	pub fn cpsrng_reseed();
	pub fn cpsrng_context_create() -> *mut CpsrngContext;
	pub fn cpsrng_context_destroy(ctx: *mut CpsrngContext);
	pub fn cpsrng_rand_bytes(ctx: *mut CpsrngContext, v: *mut u8, size: usize);

	// Only in tests
	pub fn cpsrng_test_seed(ctx: *mut CpsrngContext, iv16: *const u8, key32: *const u8);

	// Secp256k1
	pub fn secp256k1_context_create(flags: u32) -> *mut Secp256k1Context;
	pub fn secp256k1_context_destroy(ctx: *mut Secp256k1Context);

	pub fn secp256k1_ec_seckey_verify(cx: *const Secp256k1Context, sk: *const SecretKey) -> i32;
	pub fn secp256k1_ec_pubkey_create(
		cx: *const Secp256k1Context,
		pk: *mut PublicKeyUncompressed,
		sk: *const SecretKey,
	) -> i32;
	pub fn secp256k1_ec_pubkey_parse(
		cx: *const Secp256k1Context,
		pk: *mut PublicKeyUncompressed,
		input: *const PublicKey,
		intputlen: usize,
	) -> i32;
	pub fn secp256k1_ec_pubkey_combine(
		ctx: *const Secp256k1Context,
		output: *mut PublicKeyUncompressed,
		pubkeys: *const *const PublicKeyUncompressed,
		n_pubkeys: usize,
	) -> i32;
	pub fn secp256k1_ec_pubkey_serialize(
		cx: *const Secp256k1Context,
		output: *mut PublicKey,
		outputlen: *const usize,
		pubkey: *const PublicKeyUncompressed,
		flags: u32,
	) -> i32;
	pub fn secp256k1_pedersen_commitment_serialize(
		cx: *const Secp256k1Context,
		output: *mut Commitment,
		commit: *const CommitmentUncompressed,
	) -> i32;
	pub fn secp256k1_pedersen_commitment_parse(
		cx: *const Secp256k1Context,
		commit: *mut CommitmentUncompressed,
		input: *const Commitment,
	) -> i32;
	pub fn secp256k1_pedersen_commitment_to_pubkey(
		cx: *const Secp256k1Context,
		pk: *mut PublicKeyUncompressed,
		commit: *const Commitment,
	) -> i32;
	pub fn secp256k1_pedersen_commit(
		cx: *const Secp256k1Context,
		commit: *mut CommitmentUncompressed,
		blind: *const SecretKey,
		value: u64,
		value_gen: *const PublicKeyUncompressed,
		blind_gen: *const PublicKeyUncompressed,
	) -> i32;
	pub fn secp256k1_aggsig_sign_single(
		ctx: *const Secp256k1Context,
		sig: *mut Signature,
		msg32: *const Message,
		seckey32: *const SecretKey,
		secnonce32: *const SecretKey,
		extra32: *const u8,
		pubnonce_for_e: *const PublicKeyUncompressed,
		pubnonce_total: *const PublicKeyUncompressed,
		pubkey_for_e: *const PublicKeyUncompressed,
		seed32: *const u8,
	) -> i32;
	pub fn secp256k1_aggsig_verify_single(
		ctx: *const Secp256k1Context,
		sig: *const Signature,
		msg32: *const Message,
		pubnonce: *const PublicKeyUncompressed,
		pk: *const PublicKeyUncompressed,
		pk_total: *const PublicKeyUncompressed,
		extra_pubkey: *const PublicKeyUncompressed,
		is_partial: u32,
	) -> i32;

	pub fn secp256k1_aggsig_add_signatures_single(
		ctx: *const Secp256k1Context,
		ret_sig: *mut Signature,
		sigs: *const *const Signature,
		num_sigs: usize,
		pubnonce_total: *const PublicKeyUncompressed,
	) -> i32;
	pub fn secp256k1_pedersen_blind_sum(
		cx: *const Secp256k1Context,
		blind_out: *mut SecretKey,
		blinds: *const *const SecretKey,
		nblinds: usize,
		npositive: usize,
	) -> i32;
	pub fn secp256k1_pedersen_commit_sum(
		ctx: *const Secp256k1Context,
		commit_out: *const CommitmentUncompressed,
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
	pub fn secp256k1_schnorrsig_verify(
		ctx: *const Secp256k1Context,
		sig: *const Signature,
		msg32: *const Message,
		pubkey: *const PublicKeyUncompressed,
	) -> i32;
	pub fn secp256k1_scratch_space_create(
		cx: *mut Secp256k1Context,
		max_size: usize,
	) -> *mut ScratchSpace;
	pub fn secp256k1_scratch_space_destroy(sp: *mut ScratchSpace);
	pub fn secp256k1_bulletproof_rangeproof_prove(
		ctx: *const Secp256k1Context,
		scratch: *mut ScratchSpace,
		gens: *const BulletproofGenerators,
		proof: *mut u8,
		plen: *mut usize,
		tau_x: *mut u8,
		t_one: *mut PublicKeyUncompressed,
		t_two: *mut PublicKeyUncompressed,
		value: *const u64,
		min_value: *const u64,
		blind: *const *const u8,
		commits: *const *const u8,
		n_commits: usize,
		value_gen: *const PublicKeyUncompressed,
		nbits: usize,
		nonce: *const u8,
		private_nonce: *const u8,
		extra_commit: *const u8,
		extra_commit_len: usize,
		message: *const u8,
	) -> i32;
	pub fn secp256k1_bulletproof_rangeproof_verify(
		ctx: *const Secp256k1Context,
		scratch: *mut ScratchSpace,
		gens: *const BulletproofGenerators,
		proof: *const u8,
		plen: usize,
		min_value: *const u64,
		commit: *const u8,
		n_commits: usize,
		nbits: usize,
		value_gen: *const PublicKeyUncompressed,
		extra_commit: *const u8,
		extra_commit_len: usize,
	) -> i32;
	pub fn secp256k1_bulletproof_rangeproof_rewind(
		ctx: *const Secp256k1Context,
		value: *mut u64,
		blind: *mut u8,
		proof: *const u8,
		plen: usize,
		min_value: u64,
		commit: *const u8,
		value_gen: *const PublicKeyUncompressed,
		nonce: *const SecretKey,
		extra_commit: *const u8,
		extra_commit_len: usize,
		message: *mut u8,
	) -> i32;
	pub fn secp256k1_bulletproof_generators_create(
		ctx: *const Secp256k1Context,
		blinding_gen: *const PublicKeyUncompressed,
		n: usize,
	) -> *mut BulletproofGenerators;

}
