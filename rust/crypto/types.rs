use constants::{
	GENERATOR_G, GENERATOR_H, SECP256K1_EC_COMPRESSED, SECP256K1_START_SIGN,
	SECP256K1_START_VERIFY, ZERO_KEY,
};
use core::mem::size_of;
use core::ptr::{copy_nonoverlapping, null, null_mut};
use core::sync::atomic::{compiler_fence, Ordering};
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
				unsafe {
					release(ctx as *const u8);
				}
				Err(Error::new(Alloc))
			} else {
				Ok(Self { ctx, rand })
			}
		}
	}

	pub fn commit(&self, v: u64, blind: &SecretKey) -> Result<Commitment, Error> {
		// allocate for uncommpressed commitment 64 bytes
		let mut uncomp = [0u8; 64];
		// commit to v with blinding blind
		let res = unsafe {
			secp256k1_pedersen_commit(
				self.ctx,                                              // secp context
				&mut uncomp as *mut u8 as *mut CommitmentUncompressed, // dest
				blind as *const SecretKey,                             // input secret key - blinding factor
				v,                                                     // value to commit
				&GENERATOR_H as *const PublicKeyUncompressed,          // generator H
				&GENERATOR_G as *const PublicKeyUncompressed,          // generator G
			)
		};
		// check for error condition
		if res != 1 {
			Err(Error::new(Secp))
		} else {
			// compress to 33 byte format
			let mut cbytes = [0u8; 33];
			let res = unsafe {
				secp256k1_pedersen_commitment_serialize(
					self.ctx,
					&mut cbytes as *mut u8,
					&uncomp as *const u8,
				)
			};

			// check serialization error
			if res != 1 {
				Err(Error::new(Serialization))
			} else {
				Ok(Commitment(cbytes))
			}
		}
	}

	pub fn blind_sum(
		&self,
		positive: &[SecretKey],
		negative: &[SecretKey],
	) -> Result<SecretKey, Error> {
		let mut blind_out = SecretKey([0u8; 32]);
		let total_len = positive.len() + negative.len();

		// Allocate memory for an array of *const SecretKey pointers
		let ptr_size = size_of::<*const SecretKey>();
		let blinds_ptr = unsafe { alloc(total_len * ptr_size) as *mut *const SecretKey };

		if blinds_ptr.is_null() {
			return Err(Error::new(Alloc));
		}

		unsafe {
			// Populate the pointer array with positive keys
			for (i, key) in positive.iter().enumerate() {
				*blinds_ptr.add(i) = key as *const SecretKey;
			}

			// Append negative keys
			for (i, key) in negative.iter().enumerate() {
				*blinds_ptr.add(positive.len() + i) = key as *const SecretKey;
			}

			// Call the Pedersen blind sum function
			let result = secp256k1_pedersen_blind_sum(
				self.ctx,
				&mut blind_out,
				blinds_ptr,
				total_len,
				positive.len(),
			);

			// Free the allocated memory
			release(blinds_ptr as *mut u8);

			// Check result and return
			if result != 1 {
				Err(Error::new(Secp))
			} else {
				Ok(blind_out)
			}
		}
	}

	pub fn verify_balance(
		&self,
		positive: &[Commitment],
		negative: &[Commitment],
	) -> Result<bool, Error> {
		// convert slice of positive/negative commitments to vecs of uncompressed
		// commitments
		let mut pos_vec = match Vec::with_capacity(positive.len()) {
			Ok(p) => p,
			Err(e) => return Err(e),
		};
		let mut neg_vec = match Vec::with_capacity(negative.len()) {
			Ok(n) => n,
			Err(e) => return Err(e),
		};
		for p in positive {
			match p.decompress(self) {
				Ok(p) => match pos_vec.push(p) {
					Ok(_) => {}
					Err(e) => return Err(e),
				},
				Err(e) => return Err(e),
			}
		}
		for n in negative {
			match n.decompress(self) {
				Ok(n) => match neg_vec.push(n) {
					Ok(_) => {}
					Err(e) => return Err(e),
				},
				Err(e) => return Err(e),
			}
		}

		// build the slice in the expected format
		let pos_ptr_size = positive.len() * size_of::<*const u8>();
		let neg_ptr_size = negative.len() * size_of::<*const u8>();

		let pos_ptrs = unsafe { alloc(pos_ptr_size) as *mut *const u8 };

		if pos_ptrs.is_null() {
			return Err(Error::new(Alloc));
		}

		let neg_ptrs = unsafe { alloc(neg_ptr_size) as *mut *const u8 };

		if neg_ptrs.is_null() {
			unsafe {
				release(pos_ptrs as *const u8);
			}
			return Err(Error::new(Alloc));
		}

		unsafe {
			for i in 0..pos_vec.len() {
				*pos_ptrs.add(i) = &pos_vec[i].0 as *const u8;
			}
			for i in 0..neg_vec.len() {
				*neg_ptrs.add(i) = &neg_vec[i].0 as *const u8;
			}
		}

		// call secp256k1_pedersen_verify_tally to verify balance
		unsafe {
			let res = secp256k1_pedersen_verify_tally(
				self.ctx,
				pos_ptrs as *const *const CommitmentUncompressed,
				positive.len(),
				neg_ptrs as *const *const CommitmentUncompressed,
				negative.len(),
			);

			release(pos_ptrs as *const u8);
			release(neg_ptrs as *const u8);

			Ok(res == 1)
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

impl Commitment {
	fn decompress(&self, secp: &Secp) -> Result<CommitmentUncompressed, Error> {
		let mut out = [0u8; 64];
		unsafe {
			if secp256k1_pedersen_commitment_parse(
				secp.ctx,
				&mut out as *mut u8,
				&self.0 as *const u8,
			) != 1
			{
				Err(Error::new(Secp))
			} else {
				Ok(CommitmentUncompressed(out))
			}
		}
	}
}

impl Drop for SecretKey {
	fn drop(&mut self) {
		unsafe {
			copy_nonoverlapping(ZERO_KEY.as_ptr(), self.0.as_mut_ptr(), 32);
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
				let valid = secp256k1_ec_seckey_verify(secp.ctx, v.as_ptr() as *const SecretKey);
				if valid == 1 {
					break;
				}
			}
		}
		Self(v)
	}
}

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
				secret_key.0.as_ptr() as *const SecretKey,
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
				(&sig.0) as *const u8 as *const Signature,
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
				skey.0.as_ptr() as *const SecretKey,
				index,
			) != 0
		}
	}

	pub fn final_signature(&self, sig: &mut Signature) -> bool {
		unsafe {
			secp256k1_aggsig_combine_signatures(
				self.ctx,
				self.aggctx as *mut Secp256k1AggsigContext,
				(&mut sig.0) as *mut u8 as *mut Signature,
				self.partial_sigs as *mut u8 as *mut AggSigPartialSignature,
				self.nsigs,
			) != 0
		}
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
	fn test_secp_commit() {
		// create secp instance
		let secp = Secp::new().unwrap();
		// create three random blind sums
		let blind1 = SecretKey::new(&secp);
		let blind2 = SecretKey::new(&secp);
		let blind3 = SecretKey::new(&secp);

		// create their coresponding inputs/outputs with specified amounts
		let input1 = secp.commit(1000, &blind1).unwrap();
		let input2 = secp.commit(3000, &blind2).unwrap();
		let output1 = secp.commit(2000, &blind3).unwrap();

		// create blind sum that balances other sums
		let blind4 = secp.blind_sum(&[blind1, blind2], &[blind3]).unwrap();
		// create an output with this balancing factor and amount
		let output2 = secp.commit(2000, &blind4).unwrap();

		// verify balance
		assert!(secp
			.verify_balance(&[input1, input2], &[output1, output2])
			.unwrap());
	}
}
