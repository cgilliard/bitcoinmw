use core::mem::size_of;
use core::ptr::{null, null_mut};
use crypto::constants::{
	GENERATOR_G, GENERATOR_H, MAX_GENERATORS, MAX_PROOF_SIZE, SCRATCH_SPACE_SIZE,
	SECP256K1_START_SIGN, SECP256K1_START_VERIFY,
};
use crypto::cpsrng::Cpsrng;
use crypto::ffi::*;
use crypto::keys::{Message, PublicKey, PublicKeyUncompressed, SecretKey, Signature};
use crypto::pedersen::{Commitment, CommitmentUncompressed};
use crypto::range_proof::RangeProof;
use crypto::types::{BulletproofGenerators, Secp256k1Context};
use prelude::*;
use std::ffi::{alloc, release};
use std::misc::to_le_bytes_u64;

static mut SHARED_BULLETGENERATORS: Option<*mut BulletproofGenerators> = None;

pub unsafe fn shared_generators(ctx: *mut Secp256k1Context) -> *mut BulletproofGenerators {
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

pub struct Ctx {
	pub(crate) secp: *mut Secp256k1Context,
	pub(crate) rand: Cpsrng,
	sha3: Sha3,
}

impl Drop for Ctx {
	fn drop(&mut self) {
		unsafe {
			if !self.secp.is_null() {
				secp256k1_context_destroy(self.secp);
				self.secp = null_mut();
			}
		}
	}
}

impl Ctx {
	pub fn new() -> Result<Self, Error> {
		let sha3 = Sha3::new(Sha3_256)?;
		let rand = Cpsrng::new()?;
		let secp =
			unsafe { secp256k1_context_create(SECP256K1_START_SIGN | SECP256K1_START_VERIFY) };
		if secp == null_mut() {
			Err(Error::new(Alloc))
		} else {
			Ok(Self { secp, rand, sha3 })
		}
	}

	pub fn hash_kernel(
		&mut self,
		excess: &Commitment,
		fee: u64,
		features: u8,
	) -> Result<Message, Error> {
		self.sha3.reset();
		self.sha3.update(&excess.0); // 33 bytes

		let mut fee_bytes = [0u8; 8];
		to_le_bytes_u64(fee, &mut fee_bytes);
		self.sha3.update(&fee_bytes); // 8 bytes

		self.sha3.update(&[features]);

		let mut hash = [0u8; 32];
		self.sha3.finalize(&mut hash)?;
		Ok(Message(hash))
	}

	pub fn commit(&self, v: u64, blind: &SecretKey) -> Result<Commitment, Error> {
		let mut uncomp = CommitmentUncompressed([0u8; 64]);
		let res = unsafe {
			secp256k1_pedersen_commit(
				self.secp,
				uncomp.as_mut_ptr(),
				blind.as_ptr(),
				v,
				GENERATOR_H.as_ptr(),
				GENERATOR_G.as_ptr(),
			)
		};
		if res != 1 {
			Err(Error::new(InvalidCommitment))
		} else {
			let mut ret = Commitment([0u8; 33]);
			let res = unsafe {
				secp256k1_pedersen_commitment_serialize(
					self.secp,
					ret.as_mut_ptr(),
					uncomp.as_ptr(),
				)
			};

			if res != 1 {
				Err(Error::new(Serialization))
			} else {
				Ok(ret)
			}
		}
	}

	pub fn sign_single(
		&mut self,
		msg: &Message,
		seckey: &SecretKey,
		secnonce: &SecretKey,
		pubnonce: &PublicKey,
		pe: &PublicKey,
		final_nonce_sum: &PublicKey,
	) -> Result<Signature, Error> {
		let pubnonce_uncomp = pubnonce.decompress(self)?;
		let pe_uncomp = pe.decompress(self)?;
		let final_nonce_sum_uncomp = final_nonce_sum.decompress(self)?;

		self.sign_single_impl(
			msg,
			seckey,
			secnonce,
			&pubnonce_uncomp,
			&pe_uncomp,
			&final_nonce_sum_uncomp,
		)
	}

	fn sign_single_impl(
		&mut self,
		msg: &Message,
		seckey: &SecretKey,
		secnonce: &SecretKey,
		pubnonce: &PublicKeyUncompressed,
		pe: &PublicKeyUncompressed,
		final_nonce_sum: &PublicKeyUncompressed,
	) -> Result<Signature, Error> {
		let mut retsig = Signature([0u8; 64]);
		let mut seed = [0u8; 32];
		self.rand.gen(&mut seed);

		let retval = unsafe {
			secp256k1_aggsig_sign_single(
				self.secp,
				retsig.as_mut_ptr(),
				msg.as_ptr(),
				seckey.as_ptr(),
				secnonce.as_ptr(),
				null(),
				pubnonce.as_ptr(),
				final_nonce_sum.as_ptr(),
				pe.as_ptr(),
				seed.as_ptr(),
			)
		};
		if retval == 0 {
			return Err(Error::new(InvalidSignature));
		}
		Ok(retsig)
	}

	pub fn verify_single(
		&self,
		sig: &Signature,
		msg: &Message,
		pubnonce: &PublicKey,
		pubkey: &PublicKey,
		pe: &PublicKey,
		is_partial: bool,
	) -> Result<bool, Error> {
		let pubnonce_uncomp = pubnonce.decompress(self)?;
		let pe_uncomp = pe.decompress(self)?;
		let pubkey_uncomp = pubkey.decompress(self)?;
		Ok(self.verify_single_impl(
			sig,
			msg,
			&pubnonce_uncomp,
			&pubkey_uncomp,
			&pe_uncomp,
			is_partial,
		))
	}

	fn verify_single_impl(
		&self,
		sig: &Signature,
		msg: &Message,
		pubnonce: &PublicKeyUncompressed,
		pubkey: &PublicKeyUncompressed,
		pe: &PublicKeyUncompressed,
		is_partial: bool,
	) -> bool {
		let is_partial = match is_partial {
			true => 1,
			false => 0,
		};

		let retval = unsafe {
			secp256k1_aggsig_verify_single(
				self.secp,
				sig.as_ptr(),
				msg.as_ptr(),
				pubnonce.as_ptr(),
				pubkey.as_ptr(),
				pe.as_ptr(),
				null(),
				is_partial,
			)
		};
		match retval {
			0 => false,
			1 => true,
			_ => false,
		}
	}

	pub fn aggregate_signatures(
		&self,
		partial_sigs: &[&Signature],
		nonce_sum: &PublicKey,
	) -> Result<Signature, Error> {
		let nonce_sum = nonce_sum.decompress(self)?;
		let mut sig = Signature([0u8; 64]);
		let num_sigs = partial_sigs.len();
		if num_sigs > 16 {
			return Err(Error::new(TooManySignatures));
		}
		let mut sig_ptrs = [null(); 16];
		for i in 0..num_sigs {
			sig_ptrs[i] = partial_sigs[i].as_ptr();
		}
		unsafe {
			if secp256k1_aggsig_add_signatures_single(
				self.secp,
				sig.as_mut_ptr(),
				sig_ptrs.as_ptr(),
				num_sigs,
				nonce_sum.as_ptr(),
			) != 1
			{
				return Err(Error::new(InvalidSignature));
			}
		}
		Ok(sig)
	}

	pub fn blind_sum(
		&self,
		positive: &[&SecretKey],
		negative: &[&SecretKey],
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
				*blinds_ptr.add(i) = *key as *const SecretKey;
			}

			// Append negative keys
			for (i, key) in negative.iter().enumerate() {
				*blinds_ptr.add(positive.len() + i) = *key as *const SecretKey;
			}

			// Call the Pedersen blind sum function
			let result = secp256k1_pedersen_blind_sum(
				self.secp,
				&mut blind_out,
				blinds_ptr,
				total_len,
				positive.len(),
			);

			// Free the allocated memory
			release(blinds_ptr as *mut u8);

			// Check result and return
			if result != 1 {
				Err(Error::new(InvalidBlindSum))
			} else {
				Ok(blind_out)
			}
		}
	}

	pub fn verify_balance(
		&mut self,
		positive: &[&Commitment],
		negative: &[&Commitment],
		overage: i128,
	) -> Result<bool, Error> {
		if overage > 0xFFFF_FFFF_FFFF_FFFF_i128 {
			return Err(Error::new(Overflow));
		} else if overage < -0xFFFF_FFFF_FFFF_FFFF_i128 {
			return Err(Error::new(Underflow));
		}
		let mut pos_vec = match Vec::with_capacity(positive.len() + 1) {
			Ok(p) => p,
			Err(e) => return Err(e),
		};
		let mut neg_vec = match Vec::with_capacity(negative.len() + 1) {
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

		// handle overage
		if overage != 0 {
			let rblind = SecretKey::new(self);
			let p = if overage < 0 {
				self.commit((overage * -1) as u64, &rblind)?
			} else {
				self.commit(0, &rblind)?
			};
			let n = if overage > 0 {
				self.commit(overage as u64, &rblind)?
			} else {
				self.commit(0, &rblind)?
			};
			pos_vec.push(p.decompress(self)?)?;
			neg_vec.push(n.decompress(self)?)?;
		}

		// build the slice in the expected format
		let pos_ptr_size = pos_vec.len() * size_of::<*const u8>();
		let neg_ptr_size = neg_vec.len() * size_of::<*const u8>();

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
				self.secp,
				pos_ptrs as *const *const CommitmentUncompressed,
				pos_vec.len(),
				neg_ptrs as *const *const CommitmentUncompressed,
				neg_vec.len(),
			);

			release(pos_ptrs as *const u8);
			release(neg_ptrs as *const u8);

			Ok(res == 1)
		}
	}

	pub fn range_proof(&mut self, value: u64, blind: &SecretKey) -> Result<RangeProof, Error> {
		let scratch = unsafe { secp256k1_scratch_space_create(self.secp, SCRATCH_SPACE_SIZE) };
		if scratch.is_null() {
			return Err(Error::new(Alloc));
		}

		let mut proof = [0; MAX_PROOF_SIZE];
		let mut plen = MAX_PROOF_SIZE;
		let n_bits = 64;
		let extra_data_len = 0;
		let extra_data = null();
		let tau_x = null_mut();
		let t_one = null_mut();
		let t_two = null_mut();
		let commits = null_mut();
		let message_ptr = null();

		let rewind_nonce = blind.clone();
		let private_nonce = SecretKey::new(self);

		let mut blind_vec = Vec::new();
		match blind_vec.push(blind.0.as_ptr()) {
			Ok(_) => {}
			Err(e) => {
				unsafe {
					secp256k1_scratch_space_destroy(scratch);
				}
				return Err(e);
			}
		}

		unsafe {
			let res = secp256k1_bulletproof_rangeproof_prove(
				self.secp,
				scratch,
				shared_generators(self.secp),
				proof.as_mut_ptr(),
				&mut plen,
				tau_x,
				t_one,
				t_two,
				&value,
				null(), // min_values: NULL for all-zeroes
				blind_vec.as_ptr(),
				commits,
				1,
				GENERATOR_H.as_ptr(),
				n_bits,
				rewind_nonce.0.as_ptr(),
				private_nonce.0.as_ptr(),
				extra_data,
				extra_data_len,
				message_ptr,
			);
			secp256k1_scratch_space_destroy(scratch);
			if res == 0 {
				Err(Error::new(InvalidRangeProof))
			} else {
				Ok(RangeProof { proof, plen })
			}
		}
	}

	pub fn verify_range_proof(
		&mut self,
		commit: &Commitment,
		proof: &RangeProof,
	) -> Result<(), Error> {
		if proof.plen > MAX_PROOF_SIZE {
			return Err(Error::new(InvalidRangeProof));
		}

		let scratch = unsafe { secp256k1_scratch_space_create(self.secp, SCRATCH_SPACE_SIZE) };
		if scratch.is_null() {
			return Err(Error::new(Alloc));
		}

		let n_bits = 64;
		let extra_data_len = 0;
		let extra_data = null();

		let commit = match commit.decompress(self) {
			Ok(commit) => commit,
			Err(e) => {
				unsafe {
					secp256k1_scratch_space_destroy(scratch);
				}
				return Err(e);
			}
		};

		unsafe {
			let result = secp256k1_bulletproof_rangeproof_verify(
				self.secp,
				scratch,
				shared_generators(self.secp),
				proof.proof.as_ptr(),
				proof.plen,
				null(), // min_values: NULL for all-zeroes
				commit.0.as_ptr(),
				1,
				n_bits,
				GENERATOR_H.as_ptr(),
				extra_data,
				extra_data_len,
			);

			secp256k1_scratch_space_destroy(scratch);

			if result != 1 {
				Err(Error::new(InvalidRangeProof))
			} else {
				Ok(())
			}
		}
	}

	pub fn rewind_range_proof(
		&mut self,
		commit: &Commitment,
		blind: &SecretKey,
		proof: &RangeProof,
	) -> Result<u64, Error> {
		let scratch = unsafe { secp256k1_scratch_space_create(self.secp, SCRATCH_SPACE_SIZE) };
		if scratch.is_null() {
			return Err(Error::new(Alloc));
		}

		let extra_data_len = 0;
		let extra_data = null();

		let mut blind_out = [0u8; 32];
		let mut value_out = 0;
		let mut message_out = [0u8; 20];

		let commit = match commit.decompress(self) {
			Ok(commit) => commit,
			Err(e) => {
				unsafe {
					secp256k1_scratch_space_destroy(scratch);
				}
				return Err(e);
			}
		};

		unsafe {
			let result = secp256k1_bulletproof_rangeproof_rewind(
				self.secp,
				&mut value_out,
				blind_out.as_mut_ptr(),
				proof.proof.as_ptr(),
				proof.plen,
				0,
				commit.0.as_ptr(),
				GENERATOR_H.as_ptr(),
				blind.as_ptr(),
				extra_data,
				extra_data_len,
				message_out.as_mut_ptr(),
			);
			secp256k1_scratch_space_destroy(scratch);

			if result == 0 {
				Err(Error::new(InvalidRangeProof))
			} else {
				Ok(value_out)
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crypto::kernel::Kernel;

	#[test]
	fn test_simple_sign() {
		let mut secp = Ctx::new().unwrap();
		let seckey = SecretKey::new(&mut secp);
		let secnonce = SecretKey::new(&mut secp);
		let pubnonce = PublicKey::from(&mut secp, &secnonce).unwrap();
		let pubkey = PublicKey::from(&mut secp, &seckey).unwrap();

		let msg = Message([10u8; 32]);
		let sig = secp
			.sign_single(
				&msg, &seckey, &secnonce, &pubnonce, // k * G
				&pubkey,   // x * G
				&pubnonce, // k * G (single signer)
			)
			.unwrap();

		assert!(secp
			.verify_single(
				&sig, &msg, &pubnonce, // k * G
				&pubkey,   // x * G
				&pubkey,   // x * G (total for single signer)
				true       // is_partial = true
			)
			.unwrap());

		let msg = Message([11u8; 32]);
		assert!(!secp
			.verify_single(
				&sig, &msg, &pubnonce, // k * G
				&pubkey,   // x * G
				&pubkey,   // x * G (total for single signer)
				true       // is_partial = true
			)
			.unwrap());
	}

	#[test]
	fn test_aggregation_simple() {
		let mut secp = Ctx::new().unwrap();
		let msg = Message([10u8; 32]);

		// Signer 1
		let seckey1 = SecretKey::new(&mut secp);
		let secnonce1 = SecretKey::new(&mut secp);
		let pubnonce1 = PublicKey::from(&mut secp, &secnonce1).unwrap();
		let pubkey1 = PublicKey::from(&mut secp, &seckey1).unwrap();

		// Signer 2
		let seckey2 = SecretKey::new(&mut secp);
		let secnonce2 = SecretKey::new(&mut secp);
		let pubnonce2 = PublicKey::from(&mut secp, &secnonce2).unwrap();
		let pubkey2 = PublicKey::from(&mut secp, &seckey2).unwrap();

		// Sums
		let nonce_sum = pubnonce1.add(&secp, &pubnonce2).unwrap();
		let pubkey_sum = pubkey1.add(&secp, &pubkey2).unwrap();

		// Partial signatures with total sums
		let sig1 = secp
			.sign_single(
				&msg,
				&seckey1,
				&secnonce1,
				&nonce_sum,
				&pubkey_sum,
				&nonce_sum,
			)
			.unwrap();

		assert!(secp
			.verify_single(&sig1, &msg, &nonce_sum, &pubkey1, &pubkey_sum, true)
			.unwrap());

		let sig2 = secp
			.sign_single(
				&msg,
				&seckey2,
				&secnonce2,
				&nonce_sum,
				&pubkey_sum,
				&nonce_sum,
			)
			.unwrap();

		assert!(secp
			.verify_single(&sig2, &msg, &nonce_sum, &pubkey2, &pubkey_sum, true)
			.unwrap());

		// Aggregate
		let partial_sigs = &[&sig1, &sig2];
		let aggsig = secp.aggregate_signatures(partial_sigs, &nonce_sum).unwrap();

		// Verify aggregated signature (non-zero-sum)
		assert!(secp
			.verify_single(&aggsig, &msg, &nonce_sum, &pubkey_sum, &pubkey_sum, false)
			.unwrap());

		let msgbad = Message([99u8; 32]);
		assert!(!secp
			.verify_single(
				&aggsig,
				&msgbad,
				&nonce_sum,
				&pubkey_sum,
				&pubkey_sum,
				false
			)
			.unwrap());
	}

	#[test]
	fn test_balance_tx() {
		let mut secp = Ctx::new().unwrap();

		let blind1 = SecretKey::new(&mut secp);
		let input1 = secp.commit(3000, &blind1).unwrap();
		let blind2 = SecretKey::new(&mut secp);
		let change = secp.commit(1000, &blind2).unwrap();
		let blind3 = SecretKey::new(&mut secp);
		let output1 = secp.commit(2000, &blind3).unwrap();

		let excess_blind = secp.blind_sum(&[&blind1], &[&blind2, &blind3]).unwrap();
		let excess = secp.commit(0, &excess_blind).unwrap();

		assert!(secp
			.verify_balance(&[&input1], &[&change, &output1, &excess], 0)
			.unwrap());

		let output1 = secp.commit(2001, &blind3).unwrap();

		assert!(!secp
			.verify_balance(&[&input1], &[&change, &output1, &excess], 0)
			.unwrap());
	}

	#[test]
	fn test_mimblewimble_tx() {
		let mut secp = Ctx::new().unwrap();

		let blind1 = SecretKey::new(&mut secp);
		let input1 = secp.commit(3000, &blind1).unwrap();
		let blind2 = SecretKey::new(&mut secp);
		let change = secp.commit(1000, &blind2).unwrap();
		let blind3 = SecretKey::new(&mut secp);
		let output1 = secp.commit(2000, &blind3).unwrap();

		let excess_blind = secp.blind_sum(&[&blind1], &[&blind2, &blind3]).unwrap();
		let excess = secp.commit(0, &excess_blind).unwrap();
		let msg = secp.hash_kernel(&excess, 0, 0).unwrap();

		let pubkey_sum = PublicKey::from(&secp, &excess_blind).unwrap();

		let nonce1 = SecretKey::new(&mut secp);
		let pubnonce1 = PublicKey::from(&secp, &nonce1).unwrap();
		let sender_blind = secp.blind_sum(&[&blind1], &[&blind2]).unwrap();
		let pub_sender = PublicKey::from(&secp, &sender_blind).unwrap();

		let nonce2 = SecretKey::new(&mut secp);
		let pubnonce2 = PublicKey::from(&secp, &nonce2).unwrap();
		let receiver_blind = secp.blind_sum(&[], &[&blind3]).unwrap();
		let pub_receiver = PublicKey::from(&secp, &receiver_blind).unwrap();

		let nonce_sum = pubnonce1.add(&secp, &pubnonce2).unwrap();

		let sig1 = secp
			.sign_single(
				&msg,
				&sender_blind,
				&nonce1,
				&nonce_sum, // Total nonce sum for consistent e
				&pubkey_sum,
				&nonce_sum,
			)
			.unwrap();

		assert!(secp
			.verify_single(&sig1, &msg, &nonce_sum, &pub_sender, &pubkey_sum, true)
			.unwrap());

		let sig2 = secp
			.sign_single(
				&msg,
				&receiver_blind,
				&nonce2,
				&nonce_sum, // Total nonce sum for consistent e
				&pubkey_sum,
				&nonce_sum,
			)
			.unwrap();
		assert!(secp
			.verify_single(&sig2, &msg, &nonce_sum, &pub_receiver, &pubkey_sum, true)
			.unwrap());

		let partial_sigs = &[&sig1, &sig2];
		let aggsig = secp.aggregate_signatures(partial_sigs, &nonce_sum).unwrap();

		assert!(secp
			.verify_balance(&[&input1], &[&change, &output1, &excess], 0)
			.unwrap());
		assert!(secp
			.verify_single(&aggsig, &msg, &nonce_sum, &pubkey_sum, &pubkey_sum, false)
			.unwrap());
	}

	#[test]
	fn test_mimblewimble_interactive1() {
		let mut secp_send = Ctx::new().unwrap();
		let mut secp_recv = Ctx::new().unwrap();

		// start with sender
		let blind1 = SecretKey::new(&mut secp_send);
		let input = secp_send.commit(500, &blind1).unwrap();
		let blind2 = SecretKey::new(&mut secp_send);
		let change = secp_send.commit(100, &blind2).unwrap();
		let sender_excess_blind = secp_send.blind_sum(&[&blind1], &[&blind2]).unwrap();
		let sender_excess = secp_send.commit(0, &sender_excess_blind).unwrap();

		// sender generates private nonce and calcualates other values
		let sender_nonce = SecretKey::new(&mut secp_send);
		let sender_pub_nonce = PublicKey::from(&secp_send, &sender_nonce).unwrap();
		let sender_pubkey_sum = PublicKey::from(&secp_send, &sender_excess_blind).unwrap();

		// SLATE SEND 1

		// now recipient gets slate and adds outputs
		let blind3 = SecretKey::new(&mut secp_recv);
		let output1 = secp_recv.commit(201, &blind3).unwrap();
		let blind4 = SecretKey::new(&mut secp_recv);
		let output2 = secp_recv.commit(199, &blind4).unwrap();
		let recipient_excess_blind = secp_recv.blind_sum(&[], &[&blind3, &blind4]).unwrap();
		let recipient_excess = secp_recv.commit(0, &recipient_excess_blind).unwrap();

		// recipient generates private nonce
		let recipient_nonce = SecretKey::new(&mut secp_recv);
		let recipient_pub_nonce = PublicKey::from(&secp_recv, &recipient_nonce).unwrap();
		let recipient_pubkey_sum = PublicKey::from(&secp_recv, &recipient_excess_blind).unwrap();

		let pub_nonce_sum = recipient_pub_nonce
			.add(&secp_recv, &sender_pub_nonce)
			.unwrap();
		let pubkey_sum = recipient_pubkey_sum
			.add(&secp_recv, &sender_pubkey_sum)
			.unwrap();

		let excess = sender_excess.add(&secp_recv, &recipient_excess).unwrap();
		let msg = secp_recv.hash_kernel(&excess, 0, 0).unwrap();

		let sig_recv = secp_recv
			.sign_single(
				&msg,
				&recipient_excess_blind,
				&recipient_nonce,
				&pub_nonce_sum, // Total nonce sum for consistent e
				&pubkey_sum,
				&pub_nonce_sum,
			)
			.unwrap();

		assert!(secp_recv
			.verify_single(
				&sig_recv,
				&msg,
				&pub_nonce_sum,
				&recipient_pubkey_sum,
				&pubkey_sum,
				true
			)
			.unwrap());

		// SLATE SEND 2

		// no sender gets slate back and verifies things. We know the sig_recv, excess,
		// fee, features. So we can calc the msg. We have the recipient_pub_nonce and we
		// can sum it with ours to generate the pub_nonce_sum, we also get the
		// recipient_pubkey_sum and we can add it to ours to calculate the pubkey_sum

		// next we calculate our partial sig
		let sig_send = secp_send
			.sign_single(
				&msg,
				&sender_excess_blind,
				&sender_nonce,
				&pub_nonce_sum,
				&pubkey_sum,
				&pub_nonce_sum,
			)
			.unwrap();

		assert!(secp_send
			.verify_single(
				&sig_send,
				&msg,
				&pub_nonce_sum,
				&sender_pubkey_sum,
				&pubkey_sum,
				true
			)
			.unwrap());

		// now sender aggregates signatures, verifies the balance and the signatures and
		// submits to the network
		let partial_sigs = &[&sig_send, &sig_recv];
		let aggsig = secp_send
			.aggregate_signatures(partial_sigs, &pub_nonce_sum)
			.unwrap();

		assert!(secp_send
			.verify_balance(
				&[&input],
				&[
					&change,
					&output1,
					&output2,
					&recipient_excess,
					&sender_excess
				],
				0
			)
			.unwrap());

		assert!(secp_send
			.verify_single(
				&aggsig,
				&msg,
				&pub_nonce_sum,
				&pubkey_sum,
				&pubkey_sum,
				false
			)
			.unwrap());
	}

	#[test]
	fn test_mimblewimble_interactive_with_fee() -> Result<(), Error> {
		let mut secp_send = Ctx::new()?;
		let mut secp_recv = Ctx::new()?;

		let fee = 5u64;

		// Sender
		let blind1 = SecretKey::new(&mut secp_send);
		let input = secp_send.commit(500, &blind1)?;
		let blind2 = SecretKey::new(&mut secp_send);
		let change = secp_send.commit(100 - fee, &blind2)?; // Fee deducted from change (95)
		let sender_excess_blind = secp_send.blind_sum(&[&blind1], &[&blind2])?;
		let sender_excess = secp_send.commit(0, &sender_excess_blind)?; // Excess remains 0*H

		let sender_nonce = SecretKey::new(&mut secp_send);
		let sender_pub_nonce = PublicKey::from(&secp_send, &sender_nonce)?;
		let sender_pubkey_sum = PublicKey::from(&secp_send, &sender_excess_blind)?;

		// Recipient
		let blind3 = SecretKey::new(&mut secp_recv);
		let output1 = secp_recv.commit(201, &blind3)?;
		let blind4 = SecretKey::new(&mut secp_recv);
		let output2 = secp_recv.commit(199, &blind4)?; // Full amount, no fee here
		let recipient_excess_blind = secp_recv.blind_sum(&[], &[&blind3, &blind4])?;
		let recipient_excess = secp_recv.commit(0, &recipient_excess_blind)?;

		let recipient_nonce = SecretKey::new(&mut secp_recv);
		let recipient_pub_nonce = PublicKey::from(&secp_recv, &recipient_nonce)?;
		let recipient_pubkey_sum = PublicKey::from(&secp_recv, &recipient_excess_blind)?;

		let pub_nonce_sum = recipient_pub_nonce.add(&secp_recv, &sender_pub_nonce)?;
		let pubkey_sum = recipient_pubkey_sum.add(&secp_recv, &sender_pubkey_sum)?;

		let excess = sender_excess.add(&secp_recv, &recipient_excess)?; // 0*H
		let msg = secp_recv.hash_kernel(&excess, fee, 0)?;

		// Recipient signs
		let sig_recv = secp_recv.sign_single(
			&msg,
			&recipient_excess_blind,
			&recipient_nonce,
			&pub_nonce_sum,
			&pubkey_sum,
			&pub_nonce_sum,
		)?;
		assert!(secp_recv.verify_single(
			&sig_recv,
			&msg,
			&pub_nonce_sum,
			&recipient_pubkey_sum,
			&pubkey_sum,
			true
		)?);

		// Sender signs
		let sig_send = secp_send.sign_single(
			&msg,
			&sender_excess_blind,
			&sender_nonce,
			&pub_nonce_sum,
			&pubkey_sum,
			&pub_nonce_sum,
		)?;
		assert!(secp_send.verify_single(
			&sig_send,
			&msg,
			&pub_nonce_sum,
			&sender_pubkey_sum,
			&pubkey_sum,
			true
		)?);

		// Aggregate and verify
		let partial_sigs = &[&sig_send, &sig_recv];
		let aggsig = secp_send.aggregate_signatures(partial_sigs, &pub_nonce_sum)?;
		assert!(secp_send.verify_single(
			&aggsig,
			&msg,
			&pub_nonce_sum,
			&pubkey_sum,
			&pubkey_sum,
			false
		)?);

		// validate kernel
		let _kernel = Kernel::new(excess, aggsig, fee);
		//assert!(kernel.verify(&mut secp_send, &msg).is_ok());

		// Balance check with fee
		assert!(secp_send.verify_balance(
			&[&input],
			&[
				&change,
				&output1,
				&output2,
				&recipient_excess,
				&sender_excess
			],
			5
		)?);
		Ok(())
	}

	#[test]
	fn test_range_proofs() -> Result<(), Error> {
		let mut ctx = Ctx::new()?;
		let blind = SecretKey::new(&mut ctx);
		let proof = ctx.range_proof(100, &blind)?;
		let commit = ctx.commit(100, &blind)?;
		ctx.verify_range_proof(&commit, &proof)?;

		assert_eq!(ctx.rewind_range_proof(&commit, &blind, &proof)?, 100);

		let other_commit = ctx.commit(101, &blind)?;
		assert!(ctx.verify_range_proof(&other_commit, &proof).is_err());
		assert!(ctx
			.rewind_range_proof(&other_commit, &blind, &proof)
			.is_err());

		Ok(())
	}
}
