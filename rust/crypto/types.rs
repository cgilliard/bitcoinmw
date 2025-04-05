use core::iter::Iterator;
use core::mem::size_of;
use core::ops::Drop;
use core::ptr::{copy_nonoverlapping, null, null_mut, write};
use core::sync::atomic::compiler_fence;
use core::sync::atomic::Ordering;
use ffi::*;
use prelude::*;

pub struct Message([u8; 32]);

impl Message {
	pub fn new(msg: [u8; 32]) -> Self {
		Self(msg)
	}
}

pub struct Secp {
	ctx: *mut Secp256k1Context,
	rand: *mut CsprngCtx,
}

impl Secp {
	pub fn new() -> Result<Self, Error> {
		let ctx =
			unsafe { secp256k1_context_create(SECP256K1_START_SIGN | SECP256K1_START_VERIFY) };
		if ctx == null_mut() {
			Err(Error::new(Alloc))
		} else {
			let rand = unsafe { cpsrng_context_create() };
			if rand == null_mut() {
				Err(Error::new(Alloc))
			} else {
				Ok(Self { ctx, rand })
			}
		}
	}
}

impl Drop for Secp {
	fn drop(&mut self) {
		unsafe {
			if self.ctx != null_mut() {
				secp256k1_context_destroy(self.ctx);
				self.ctx = null_mut();
			}
			if self.rand != null_mut() {
				cpsrng_context_destroy(self.rand);
				self.rand = null_mut();
			}
		}
	}
}

pub struct SecretKey([u8; 32]);

const ZERO: [u8; 32] = [0; 32];
impl Drop for SecretKey {
	fn drop(&mut self) {
		unsafe {
			copy_nonoverlapping(ZERO.as_ptr(), self.0.as_mut_ptr(), 32);
			compiler_fence(Ordering::SeqCst);
		}
	}
}

impl SecretKey {
	pub fn new(secp: &Secp) -> Self {
		let mut v = [0u8; 32];
		loop {
			unsafe {
				cpsrng_rand_bytes(secp.rand, v.as_mut_ptr(), 32);
				let valid =
					secp256k1_ec_seckey_verify(secp.ctx, v.as_ptr() as *const SecretKeyImpl);
				if valid == 1 {
					break;
				}
			}
		}
		Self(v)
	}
}

pub struct PublicKey([u8; 33]);

pub struct Signature([u8; 64]);

impl Signature {
	pub fn new() -> Self {
		Self([0u8; 64])
	}
}

impl PublicKey {
	pub fn from(secp: &Secp, secret_key: &SecretKey) -> Result<Self, Error> {
		let mut v = [0u8; 33];
		let mut uncomp = [0u8; 64];

		unsafe {
			if secp256k1_ec_pubkey_create(
				secp.ctx,
				uncomp.as_mut_ptr() as *mut PublicKeyUncompressed,
				secret_key.0.as_ptr() as *const SecretKeyImpl,
			) == 0
			{
				return Err(Error::new(Secp));
			}

			let mut len = 33usize;
			let serialize_result = secp256k1_ec_pubkey_serialize(
				secp.ctx,
				v.as_mut_ptr(),
				&mut len,
				&uncomp as *const u8 as *const PublicKeyUncompressed,
				SECP256K1_EC_COMPRESSED,
			);
			if serialize_result == 0 {
				Err(Error::new(Serialization))
			} else {
				Ok(Self(v))
			}
		}
	}
}

pub struct AggSig {
	ctx: *const Secp256k1Context,
	aggctx: *const Secp256k1AggsigContext,
	partial_sigs: *const u8,
	pkeyarr: *const u8,
	nsigs: usize,
}

impl Drop for AggSig {
	fn drop(&mut self) {
		unsafe {
			if self.pkeyarr != null_mut() {
				release(self.pkeyarr);
			}
			if self.partial_sigs != null_mut() {
				release(self.partial_sigs);
			}
		}
	}
}

impl AggSig {
	pub fn new(secp: &Secp, pkeys: &[PublicKey]) -> Result<Self, Error> {
		unsafe {
			let pkeyarr = alloc(pkeys.len() * 64);
			if pkeyarr == null() {
				return Err(Error::new(Alloc));
			}
			let mut i = 0;
			for pkey in pkeys {
				let mut pkey_uncomp = [0u8; 64];
				if secp256k1_ec_pubkey_parse(
					secp.ctx,
					&mut pkey_uncomp as *mut u8 as *mut PublicKeyUncompressed,
					pkey.0.as_ptr(),
					33,
				) == 0
				{
					release(pkeyarr);
					return Err(Error::new(Secp));
				}
				copy_nonoverlapping(pkey_uncomp.as_ptr(), pkeyarr.add(64 * i) as *mut u8, 64);
				i += 1;
			}
			let mut seed = [0u8; 32];
			cpsrng_rand_bytes(secp.rand, seed.as_mut_ptr(), 32);
			let aggctx = secp256k1_aggsig_context_create(
				secp.ctx,
				pkeyarr as *const u8 as *const PublicKeyUncompressed,
				pkeys.len(),
				seed.as_ptr(),
			);
			if aggctx == null_mut() {
				release(pkeyarr);
				return Err(Error::new(Secp));
			}
			let partial_sigs = alloc(32 * pkeys.len());
			if partial_sigs == null() {
				release(pkeyarr);
				return Err(Error::new(Alloc));
			}
			let nsigs = pkeys.len();
			Ok(Self {
				ctx: secp.ctx,
				aggctx,
				partial_sigs,
				pkeyarr,
				nsigs,
			})
		}
	}

	pub fn verify(&self, msg: &Message, sig: &Signature) -> bool {
		unsafe {
			secp256k1_aggsig_build_scratch_and_verify(
				self.ctx,
				(&sig.0) as *const u8 as *const SignatureImpl,
				&msg.0 as *const u8,
				self.pkeyarr as *const u8 as *const PublicKeyUncompressed,
				self.nsigs,
			) == 1
		}
	}

	pub fn gen_nonce(&self, index: usize) -> bool {
		unsafe {
			secp256k1_aggsig_generate_nonce(
				self.ctx,
				self.aggctx as *mut Secp256k1AggsigContext,
				index,
			) != 0
		}
	}

	pub fn partial_sign(&self, index: usize, msg: &Message, skey: &SecretKey) -> bool {
		unsafe {
			secp256k1_aggsig_partial_sign(
				self.ctx,
				self.aggctx as *mut Secp256k1AggsigContext,
				self.partial_sigs.add(index * 32) as *mut u8 as *mut AggSigPartialSignature,
				&msg.0 as *const u8,
				skey.0.as_ptr() as *const SecretKeyImpl,
				index,
			) != 0
		}
	}

	pub fn final_signature(&self, sig: &mut Signature) -> bool {
		unsafe {
			secp256k1_aggsig_combine_signatures(
				self.ctx,
				self.aggctx as *mut Secp256k1AggsigContext,
				(&mut sig.0) as *mut u8 as *mut SignatureImpl,
				self.partial_sigs as *mut u8 as *mut AggSigPartialSignature,
				self.nsigs,
			) != 0
		}
	}
}

pub struct Commitment([u8; 33]);

impl Commitment {
	pub fn new(secp: &Secp, v: u64, blind: &SecretKey) -> Result<Self, Error> {
		let mut uncomp = [0u8; 64];
		let mut ret = [0u8; 33];
		unsafe {
			if secp256k1_pedersen_commit(
				secp.ctx,
				&mut uncomp as *mut u8 as *mut CommitmentUncompressed,
				&blind.0 as *const u8 as *const SecretKeyImpl,
				v,
				&GENERATOR_H as *const PublicKeyUncompressed,
				&GENERATOR_G as *const PublicKeyUncompressed,
			) == 0
			{
				return Err(Error::new(Secp));
			}
		}

		let mut outputlen = 33usize;
		unsafe {
			let success = secp256k1_ec_pubkey_serialize(
				secp.ctx,
				ret.as_mut_ptr(),
				&mut outputlen as *mut usize,
				uncomp.as_ptr() as *const PublicKeyUncompressed,
				SECP256K1_SER_COMPRESSED,
			);
			if success != 1 || outputlen != 33 {
				return Err(Error::new(Secp));
			}
		}

		Ok(Self(ret))
	}

	pub fn to_uncompressed(&self, secp: &Secp) -> Result<CommitmentUncompressed, Error> {
		let mut ret = [0u8; 64];
		unsafe {
			if secp256k1_ec_pubkey_parse(
				secp.ctx,
				&mut ret as *mut u8 as *mut PublicKeyUncompressed,
				self.0.as_ptr(),
				33,
			) != 1
			{
				Err(Error::new(Secp))
			} else {
				Ok(CommitmentUncompressed(ret))
			}
		}
	}

	pub fn balance(secp: &Secp, positive: &[Commitment], negative: &[Commitment]) -> bool {
		// Allocate uncompressed arrays
		let pos_uncomp_array = unsafe { alloc(positive.len() * 64) };
		if pos_uncomp_array.is_null() {
			return false;
		}
		let neg_uncomp_array = unsafe { alloc(negative.len() * 64) };
		if neg_uncomp_array.is_null() {
			unsafe {
				release(pos_uncomp_array);
			}
			return false;
		}

		// Decompress positive commitments
		for (i, commit) in positive.iter().enumerate() {
			let offset = i * 64;
			let uncomp_ptr = unsafe { pos_uncomp_array.add(offset) as *mut CommitmentUncompressed };
			unsafe {
				if secp256k1_ec_pubkey_parse(
					secp.ctx,
					uncomp_ptr as *mut PublicKeyUncompressed,
					commit.0.as_ptr(),
					33,
				) != 1
				{
					release(pos_uncomp_array);
					release(neg_uncomp_array);
					return false;
				}
			}
		}

		// Decompress negative commitments (fixed to use neg_uncomp_array)
		for (i, commit) in negative.iter().enumerate() {
			let offset = i * 64;
			let uncomp_ptr = unsafe { neg_uncomp_array.add(offset) as *mut CommitmentUncompressed };
			unsafe {
				if secp256k1_ec_pubkey_parse(
					secp.ctx,
					uncomp_ptr as *mut PublicKeyUncompressed,
					commit.0.as_ptr(),
					33,
				) != 1
				{
					release(pos_uncomp_array);
					release(neg_uncomp_array);
					return false;
				}
			}
		}

		// Allocate pointer arrays (fixed size)
		let ptr_size = size_of::<*const CommitmentUncompressed>();
		let pos_ptr_array =
			unsafe { alloc(positive.len() * ptr_size) as *mut *const CommitmentUncompressed };
		if pos_ptr_array.is_null() {
			unsafe {
				release(pos_uncomp_array);
				release(neg_uncomp_array);
			}
			return false;
		}
		let neg_ptr_array =
			unsafe { alloc(negative.len() * ptr_size) as *mut *const CommitmentUncompressed };
		if neg_ptr_array.is_null() {
			unsafe {
				release(pos_uncomp_array);
				release(neg_uncomp_array);
				release(pos_ptr_array as *mut u8);
			}
			return false;
		}

		// Fill pointer arrays
		for i in 0..positive.len() {
			let uncomp_offset = i * 64;
			unsafe {
				let uncomp_ptr =
					pos_uncomp_array.add(uncomp_offset) as *const CommitmentUncompressed;
				write(pos_ptr_array.add(i), uncomp_ptr);
			}
		}
		for i in 0..negative.len() {
			let uncomp_offset = i * 64;
			unsafe {
				let uncomp_ptr =
					neg_uncomp_array.add(uncomp_offset) as *const CommitmentUncompressed;
				write(neg_ptr_array.add(i), uncomp_ptr);
			}
		}

		// Verify balance and clean up
		let result = unsafe {
			secp256k1_pedersen_verify_tally(
				secp.ctx,
				pos_ptr_array,
				positive.len(),
				neg_ptr_array,
				negative.len(),
			) == 1
		};

		unsafe {
			release(pos_ptr_array as *mut u8);
			release(neg_ptr_array as *mut u8);
			release(pos_uncomp_array);
			release(neg_uncomp_array);
		}

		result
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_signing() {
		let secp = Secp::new().unwrap();
		let skey1 = SecretKey::new(&secp);
		let pkey1 = PublicKey::from(&secp, &skey1).unwrap();
		let skey2 = SecretKey::new(&secp);
		let pkey2 = PublicKey::from(&secp, &skey2).unwrap();

		let pkeys = [pkey1, pkey2];
		let aggsig = AggSig::new(&secp, &pkeys).unwrap();
		assert!(aggsig.gen_nonce(0));
		assert!(aggsig.gen_nonce(1));
		let msg32 = Message::new([7u8; 32]);
		assert!(aggsig.partial_sign(0, &msg32, &skey1));
		assert!(aggsig.partial_sign(1, &msg32, &skey2));

		let mut sig = Signature::new();
		aggsig.final_signature(&mut sig);
		assert!(aggsig.verify(&msg32, &sig));
		let msgbad = Message::new([8u8; 32]);
		assert!(!aggsig.verify(&msgbad, &sig));
	}

	#[test]
	fn test_signing_multi() {
		let secp = Secp::new().unwrap();
		let skey1 = SecretKey::new(&secp);
		let pkey1 = PublicKey::from(&secp, &skey1).unwrap();
		let skey2 = SecretKey::new(&secp);
		let pkey2 = PublicKey::from(&secp, &skey2).unwrap();
		let skey3 = SecretKey::new(&secp);
		let pkey3 = PublicKey::from(&secp, &skey3).unwrap();

		let pkeys = [pkey1, pkey2, pkey3];
		let aggsig = AggSig::new(&secp, &pkeys).unwrap();
		assert!(aggsig.gen_nonce(0));
		assert!(aggsig.gen_nonce(1));
		assert!(aggsig.gen_nonce(2));
		let msg32 = Message::new([9u8; 32]);
		assert!(aggsig.partial_sign(0, &msg32, &skey1));
		assert!(aggsig.partial_sign(1, &msg32, &skey2));
		assert!(aggsig.partial_sign(2, &msg32, &skey3));

		let mut sig = Signature::new();
		aggsig.final_signature(&mut sig);
		assert!(aggsig.verify(&msg32, &sig));
		let msgbad = Message::new([88u8; 32]);
		assert!(!aggsig.verify(&msgbad, &sig));
	}

	#[test]
	fn test_commitment_ser() {
		let secp = Secp::new().unwrap();
		let blind1 = SecretKey::new(&secp);
		let blind2 = SecretKey::new(&secp);
		let blind3 = SecretKey::new(&secp);
		let mut blind4 = SecretKey([0u8; 32]);
		let input1 = Commitment::new(&secp, 1000, &blind1).unwrap();
		//let input1_uncomp = input1.to_uncompressed(&secp).unwrap();
	}

	#[test]
	fn test_commitments() {
		unsafe {
			let secp = Secp::new().unwrap();
			let r = secp.rand;
			let iv = [0u8; 16];
			let key = [2u8; 32];
			cpsrng_test_seed(r, iv.as_ptr(), key.as_ptr());

			let blind1 = SecretKey::new(&secp);
			let blind2 = SecretKey::new(&secp);
			let blind3 = SecretKey::new(&secp);
			let mut blind4 = SecretKey([0u8; 32]);

			// Compute blind4 = blind1 + blind2 - blind3
			let mut sum_in = blind1.0; // Copy blind1
			assert_eq!(
				secp256k1_ec_privkey_tweak_add(
					secp.ctx,
					sum_in.as_mut_ptr() as *mut SecretKeyImpl,
					blind2.0.as_ptr() as *mut SecretKeyImpl,
				),
				1,
				"Tweak add failed for input blinds"
			);
			let blinds = [
				sum_in.as_ptr() as *const SecretKeyImpl,
				blind3.0.as_ptr() as *const SecretKeyImpl,
			];
			assert_eq!(
				secp256k1_pedersen_blind_sum(
					secp.ctx,
					blind4.0.as_mut_ptr() as *mut SecretKeyImpl,
					blinds.as_ptr(),
					2,
					1,
				),
				1,
				"Blind sum failed for output blind"
			);

			/*
			let mut input1 = CommitmentUncompressed([0u8; 64]);
			let mut input2 = CommitmentUncompressed([0u8; 64]);
			let mut output1 = CommitmentUncompressed([0u8; 64]);
			let mut output2 = CommitmentUncompressed([0u8; 64]);
			assert_eq!(
				secp256k1_pedersen_commit(
					secp.ctx,
					&mut input1 as *mut CommitmentUncompressed,
					&blind1.0 as *const u8 as *const SecretKeyImpl,
					1000,
					&GENERATOR_H as *const PublicKeyUncompressed,
					&GENERATOR_G as *const PublicKeyUncompressed
				),
				1
			);
			assert_eq!(
				secp256k1_pedersen_commit(
					secp.ctx,
					&mut input2 as *mut CommitmentUncompressed,
					&blind2.0 as *const u8 as *const SecretKeyImpl,
					3000,
					&GENERATOR_H as *const PublicKeyUncompressed,
					&GENERATOR_G as *const PublicKeyUncompressed
				),
				1
			);
			assert_eq!(
				secp256k1_pedersen_commit(
					secp.ctx,
					&mut output1 as *mut CommitmentUncompressed,
					&blind3.0 as *const u8 as *const SecretKeyImpl,
					2000,
					&GENERATOR_H as *const PublicKeyUncompressed,
					&GENERATOR_G as *const PublicKeyUncompressed
				),
				1
			);
			assert_eq!(
				secp256k1_pedersen_commit(
					secp.ctx,
					&mut output2 as *mut CommitmentUncompressed,
					&blind4.0 as *const u8 as *const SecretKeyImpl,
					2000,
					&GENERATOR_H as *const PublicKeyUncompressed,
					&GENERATOR_G as *const PublicKeyUncompressed
				),
				1
			);
						*/

			let input1 = Commitment::new(&secp, 1000, &blind1).unwrap();
			let input2 = Commitment::new(&secp, 3000, &blind2).unwrap();
			let output1 = Commitment::new(&secp, 2000, &blind3).unwrap();
			let output2 = Commitment::new(&secp, 2000, &blind4).unwrap();

			/*
			let positive = [
				&input1.0 as *const u8 as *const CommitmentUncompressed,
				&input2.0 as *const u8 as *const CommitmentUncompressed,
			];
			let negative = [
				&output1.0 as *const u8 as *const CommitmentUncompressed,
				&output2.0 as *const u8 as *const CommitmentUncompressed,
			];
			assert_eq!(
				secp256k1_pedersen_verify_tally(
					secp.ctx,
					positive.as_ptr(),
					2,
					negative.as_ptr(),
					2
				),
				1
			);
						*/

			/*
			let input1 = Commitment::new(&secp, 1000, &blind1).unwrap();
			let input2 = Commitment::new(&secp, 3000, &blind2).unwrap();
			let output1 = Commitment::new(&secp, 2000, &blind3).unwrap();
			let output2 = Commitment::new(&secp, 2000, &blind4).unwrap();
						*/

			/*
			let positive = [input1, input2];
			let negative = [output1, output2];
			let balanced = Commitment::balance(&secp, &positive, &negative);
			assert!(balanced);
						*/
		}
	}
}
