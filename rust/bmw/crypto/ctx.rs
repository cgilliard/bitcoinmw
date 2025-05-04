use core::mem::size_of;
use core::ptr::{null, null_mut};
use crypto::constants::{
	BLIND_SUM_MAX_KEYS, GENERATOR_G, GENERATOR_H, MAX_AGGREGATE_SIGNATURES, MAX_GENERATORS,
	MAX_PROOF_SIZE, SCRATCH_SPACE_SIZE, SECP256K1_START_SIGN, SECP256K1_START_VERIFY,
};
use crypto::cpsrng::Cpsrng;
use crypto::errors::*;
use crypto::ffi::{
	secp256k1_aggsig_add_signatures_single, secp256k1_aggsig_sign_single,
	secp256k1_aggsig_verify_single, secp256k1_bulletproof_generators_create,
	secp256k1_bulletproof_generators_destroy, secp256k1_bulletproof_rangeproof_prove,
	secp256k1_bulletproof_rangeproof_rewind, secp256k1_bulletproof_rangeproof_verify,
	secp256k1_context_create, secp256k1_context_destroy, secp256k1_pedersen_blind_sum,
	secp256k1_pedersen_commit, secp256k1_pedersen_commitment_serialize,
	secp256k1_pedersen_verify_tally, secp256k1_scratch_space_create,
	secp256k1_scratch_space_destroy,
};
use crypto::keys::{PublicKey, PublicKeyUncompressed, SecretKey};
use crypto::pedersen::{Commitment, CommitmentUncompressed};
use crypto::range_proof::RangeProof;
use crypto::signature::{Message, Signature};
use crypto::types::{BulletproofGenerators, Secp256k1Context};
use ffi::{alloc, release};
use prelude::*;

pub struct Ctx {
	secp: *const Secp256k1Context,
	rng: Cpsrng,
	gens: *mut BulletproofGenerators,
}

impl AsRaw<Secp256k1Context> for Ctx {
	fn as_ptr(&self) -> *const Secp256k1Context {
		self.secp
	}
}
impl AsRawMut<Secp256k1Context> for Ctx {
	fn as_mut_ptr(&mut self) -> *mut Secp256k1Context {
		self.secp as *mut Secp256k1Context
	}
}

impl Drop for Ctx {
	fn drop(&mut self) {
		unsafe {
			if !self.gens.is_null() {
				secp256k1_bulletproof_generators_destroy(self.secp, self.gens);
				self.gens = null_mut();
			}
			if !self.secp.is_null() {
				secp256k1_context_destroy(self.secp);
				self.secp = null_mut();
			}
		}
	}
}

impl Ctx {
	pub fn new() -> Result<Self> {
		let rng = Cpsrng::new()?;
		let secp =
			unsafe { secp256k1_context_create(SECP256K1_START_SIGN | SECP256K1_START_VERIFY) };
		if secp == null_mut() {
			err!(Alloc)
		} else {
			let gens = unsafe {
				secp256k1_bulletproof_generators_create(
					secp,
					&GENERATOR_G as *const PublicKeyUncompressed,
					MAX_GENERATORS,
				)
			};
			if gens.is_null() {
				unsafe {
					secp256k1_context_destroy(secp);
				}
				return err!(Alloc);
			}
			Ok(Self { secp, rng, gens })
		}
	}

	#[cfg(test)]
	pub fn seed(key: [u8; 32], iv: [u8; 16]) -> Result<Self> {
		let rng = Cpsrng::test_seed(key, iv)?;
		let secp =
			unsafe { secp256k1_context_create(SECP256K1_START_SIGN | SECP256K1_START_VERIFY) };
		if secp == null_mut() {
			err!(Alloc)
		} else {
			let gens = unsafe {
				secp256k1_bulletproof_generators_create(
					secp,
					&GENERATOR_G as *const PublicKeyUncompressed,
					MAX_GENERATORS,
				)
			};
			if gens.is_null() {
				unsafe {
					secp256k1_context_destroy(secp);
				}
				return err!(Alloc);
			}
			Ok(Self { secp, rng, gens })
		}
	}

	pub fn gen(&self, b: &mut [u8]) {
		self.rng.gen(b);
	}

	pub fn commit(&self, v: u64, blind: &SecretKey) -> Result<Commitment> {
		let mut uncomp = CommitmentUncompressed::zero();
		let res = unsafe {
			secp256k1_pedersen_commit(
				self.as_ptr(),
				uncomp.as_mut_ptr(),
				blind.as_ptr(),
				v,
				GENERATOR_H.as_ptr(),
				GENERATOR_G.as_ptr(),
			)
		};
		if res != 1 {
			err!(OperationFailed)
		} else {
			let mut ret = Commitment::zero();
			let res = unsafe {
				secp256k1_pedersen_commitment_serialize(
					self.as_ptr(),
					ret.as_mut_ptr(),
					uncomp.as_ptr(),
				)
			};

			if res != 1 {
				err!(Serialization)
			} else {
				Ok(ret)
			}
		}
	}

	pub fn sign(
		&self,
		msg: &Message,
		seckey: &SecretKey,
		secnonce: &SecretKey,
		pubnonce: &PublicKey,
		pe: &PublicKey,
	) -> Result<Signature> {
		let pubnonce = pubnonce.decompress(self)?;
		let pe = pe.decompress(self)?;

		let mut retsig = Signature::new();
		let mut seed = [0u8; 32];
		self.gen(&mut seed);

		let retval = unsafe {
			secp256k1_aggsig_sign_single(
				self.secp,
				retsig.as_mut_ptr(),
				msg.as_ptr(),
				seckey.as_ptr(),
				secnonce.as_ptr(),
				null(),
				pubnonce.as_ptr(),
				pubnonce.as_ptr(),
				pe.as_ptr(),
				seed.as_ptr(),
			)
		};
		if retval == 0 {
			return err!(OperationFailed);
		}
		Ok(retsig)
	}

	pub fn verify(
		&self,
		sig: &Signature,
		msg: &Message,
		pubnonce: &PublicKey,
		pubkey: &PublicKey,
		pe: &PublicKey,
		is_partial: bool,
	) -> Result<()> {
		let pubnonce = pubnonce.decompress(self)?;
		let pe = pe.decompress(self)?;
		let pubkey = pubkey.decompress(self)?;

		let is_partial = match is_partial {
			true => 1,
			false => 0,
		};

		unsafe {
			match {
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
			} {
				1 => Ok(()),
				_ => err!(ValidationFailed),
			}
		}
	}

	pub fn aggregate_signatures(
		&self,
		partial_sigs: &[&Signature],
		nonce_sum: &PublicKey,
	) -> Result<Signature> {
		let nonce_sum = nonce_sum.decompress(self)?;
		let mut sig = Signature::new();
		let num_sigs = partial_sigs.len();
		if num_sigs > MAX_AGGREGATE_SIGNATURES {
			return err!(IllegalArgument);
		}
		let mut sig_ptrs = [null(); 16];
		for i in 0..num_sigs {
			sig_ptrs[i] = partial_sigs[i].as_ptr();
		}
		unsafe {
			if secp256k1_aggsig_add_signatures_single(
				self.as_ptr(),
				sig.as_mut_ptr(),
				sig_ptrs.as_ptr(),
				num_sigs,
				nonce_sum.as_ptr(),
			) != 1
			{
				return err!(OperationFailed);
			}
		}
		Ok(sig)
	}

	pub fn blind_sum(&self, positive: &[&SecretKey], negative: &[&SecretKey]) -> Result<SecretKey> {
		let total_len = positive.len() + negative.len();

		if total_len > BLIND_SUM_MAX_KEYS {
			return err!(IllegalArgument);
		}

		let mut blinds: [*const SecretKey; BLIND_SUM_MAX_KEYS] = [null(); BLIND_SUM_MAX_KEYS];

		for (i, key) in positive.iter().enumerate() {
			if i < blinds.len() {
				blinds[i] = key.as_ptr();
			}
		}

		for (i, key) in negative.iter().enumerate() {
			let index = positive.len() + i;
			if index < blinds.len() {
				blinds[index] = key.as_ptr();
			}
		}

		let mut blind_out = SecretKey::zero();
		let result = unsafe {
			secp256k1_pedersen_blind_sum(
				self.secp,
				blind_out.as_mut_ptr(),
				blinds.as_ptr(),
				total_len,
				positive.len(),
			)
		};

		if result != 1 {
			err!(OperationFailed)
		} else {
			Ok(blind_out)
		}
	}

	// verify balance using references.
	pub fn verify_balance(
		&self,
		positive: &[&Commitment],
		negative: &[&Commitment],
		overage: i128,
	) -> Result<()> {
		// Convert &[&Commitment] to Vec<CommitmentUncompressed>
		let mut pos_vec = match Vec::with_capacity(positive.len() + 1) {
			Ok(p) => p,
			Err(e) => return Err(e),
		};
		let mut neg_vec = match Vec::with_capacity(negative.len() + 1) {
			Ok(n) => n,
			Err(e) => return Err(e),
		};

		for &p in positive {
			pos_vec.push(p.decompress(self)?)?;
		}
		for &n in negative {
			neg_vec.push(n.decompress(self)?)?;
		}

		// Delegate to shared helper
		self.verify_balance_impl(pos_vec, neg_vec, overage)
	}

	// verify for owned Commitments.
	pub fn verify_balance_owned(
		&self,
		positive: &[Commitment],
		negative: &[Commitment],
		overage: i128,
	) -> Result<()> {
		// Convert &[Commitment] to Vec<CommitmentUncompressed>
		let mut pos_vec = match Vec::with_capacity(positive.len() + 1) {
			Ok(p) => p,
			Err(e) => return Err(e),
		};
		let mut neg_vec = match Vec::with_capacity(negative.len() + 1) {
			Ok(n) => n,
			Err(e) => return Err(e),
		};

		for p in positive {
			pos_vec.push(p.decompress(self)?)?;
		}
		for n in negative {
			neg_vec.push(n.decompress(self)?)?;
		}

		// Delegate to shared helper
		self.verify_balance_impl(pos_vec, neg_vec, overage)
	}

	// Shared helper for balance verification.
	fn verify_balance_impl(
		&self,
		pos_vec: Vec<CommitmentUncompressed>,
		neg_vec: Vec<CommitmentUncompressed>,
		overage: i128,
	) -> Result<()> {
		// Validate overage
		if overage > 0xFFFF_FFFF_FFFF_FFFF_i128 {
			return err!(Overflow);
		} else if overage < -0xFFFF_FFFF_FFFF_FFFF_i128 {
			return err!(Underflow);
		}

		// Mutable copies to handle overage
		let mut pos_vec = pos_vec;
		let mut neg_vec = neg_vec;

		// Handle overage
		if overage != 0 {
			let rblind = SecretKey::gen(self);
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

		// Build pointer arrays
		let pos_ptr_size = pos_vec.len() * size_of::<*const u8>();
		let neg_ptr_size = neg_vec.len() * size_of::<*const u8>();

		let pos_ptrs = unsafe { alloc(pos_ptr_size) as *mut *const u8 };
		if pos_ptrs.is_null() {
			return err!(Alloc);
		}

		let neg_ptrs = unsafe { alloc(neg_ptr_size) as *mut *const u8 };
		if neg_ptrs.is_null() {
			unsafe {
				release(pos_ptrs as *const u8);
			}
			return err!(Alloc);
		}

		unsafe {
			for i in 0..pos_vec.len() {
				*pos_ptrs.add(i) = pos_vec[i].as_ptr() as *const u8;
			}
			for i in 0..neg_vec.len() {
				*neg_ptrs.add(i) = neg_vec[i].as_ptr() as *const u8;
			}
		}

		// Verify balance
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

			if res == 1 {
				Ok(())
			} else {
				err!(ValidationFailed)
			}
		}
	}

	pub fn range_proof(&self, value: u64, blind: &SecretKey) -> Result<RangeProof> {
		let mut proof = [0; MAX_PROOF_SIZE];
		let mut plen = MAX_PROOF_SIZE;
		let n_bits = 64;
		let private_nonce = SecretKey::gen(self);

		// Create a stack-allocated array containing the pointer to blind
		let blind_ptr = blind.as_ptr();
		let blind_array = [blind_ptr]; // Array of one pointer

		unsafe {
			let scratch = secp256k1_scratch_space_create(self.secp, SCRATCH_SPACE_SIZE);
			if scratch.is_null() {
				return err!(Alloc);
			}

			let res = secp256k1_bulletproof_rangeproof_prove(
				self.secp,
				scratch,
				self.gens,
				proof.as_mut_ptr(),
				&mut plen,
				null_mut(), // tau_x
				null_mut(), // t_one
				null_mut(), // t_two
				&value,
				null(),               // min_values
				blind_array.as_ptr(), // Pass pointer to array of blind pointers
				null_mut(),           // commits
				1,                    // Number of commitments
				GENERATOR_H.as_ptr(),
				n_bits,
				blind.as_ptr(), // rewind_nonce
				private_nonce.as_ptr(),
				null(), // extra_data
				0,      // extra_data_len
				null(), // message_ptr
			);

			secp256k1_scratch_space_destroy(scratch);

			if res == 0 {
				return err!(OperationFailed);
			}
			if plen > MAX_PROOF_SIZE {
				return err!(IllegalState);
			}

			Ok(RangeProof { proof, plen })
		}
	}

	pub fn verify_range_proof(&self, commit: &Commitment, proof: &RangeProof) -> Result<()> {
		if proof.plen > MAX_PROOF_SIZE {
			return err!(IllegalArgument);
		}

		let n_bits = 64;
		let extra_data_len = 0;
		let extra_data = null();

		let commit = commit.decompress(self)?;

		unsafe {
			let scratch = secp256k1_scratch_space_create(self.secp, SCRATCH_SPACE_SIZE);
			if scratch.is_null() {
				return err!(Alloc);
			}
			let result = secp256k1_bulletproof_rangeproof_verify(
				self.secp,
				scratch,
				self.gens,
				proof.proof.as_ptr(),
				proof.plen,
				null(), // min_values: NULL for all-zeroes
				commit.as_ptr(),
				1,
				n_bits,
				GENERATOR_H.as_ptr(),
				extra_data,
				extra_data_len,
			);

			secp256k1_scratch_space_destroy(scratch);

			if result != 1 {
				err!(OperationFailed)
			} else {
				Ok(())
			}
		}
	}

	pub fn rewind_range_proof(
		&self,
		commit: &Commitment,
		blind: &SecretKey,
		proof: &RangeProof,
	) -> Result<u64> {
		if proof.plen > MAX_PROOF_SIZE || proof.plen == 0 {
			return err!(IllegalArgument);
		}

		let extra_data_len = 0;
		let extra_data = null();

		let mut blind_out = [0u8; 32];
		let mut value_out = 0;
		let mut message_out = [0u8; 20];

		let commit = commit.decompress(self)?;

		unsafe {
			let scratch = secp256k1_scratch_space_create(self.secp, SCRATCH_SPACE_SIZE);
			if scratch.is_null() {
				return err!(Alloc);
			}

			let result = secp256k1_bulletproof_rangeproof_rewind(
				self.secp,
				&mut value_out,
				blind_out.as_mut_ptr(),
				proof.proof.as_ptr(),
				proof.plen,
				0, // min_value
				commit.as_ptr(),
				GENERATOR_H.as_ptr(),
				blind.as_ptr(),
				extra_data,
				extra_data_len,
				message_out.as_mut_ptr(),
			);

			secp256k1_scratch_space_destroy(scratch);

			if result == 0 {
				return err!(OperationFailed);
			}

			Ok(value_out)
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_simple_sign() -> Result<()> {
		let mut secp = Ctx::new()?;
		let seckey = SecretKey::gen(&mut secp);
		let secnonce = SecretKey::gen(&mut secp);
		let pubnonce = PublicKey::from(&mut secp, &secnonce)?;
		let pubkey = PublicKey::from(&mut secp, &seckey)?;

		let msg = Message::new([10u8; 32]);
		let sig = secp.sign(&msg, &seckey, &secnonce, &pubnonce, &pubkey)?;

		assert!(secp
			.verify(&sig, &msg, &pubnonce, &pubkey, &pubkey, true)
			.is_ok());

		let msg = Message::new([11u8; 32]);
		assert!(!secp
			.verify(&sig, &msg, &pubnonce, &pubkey, &pubkey, true)
			.is_ok());

		Ok(())
	}

	#[test]
	fn test_aggregation_simple() -> Result<()> {
		let mut secp = Ctx::new()?;
		let msg = Message::new([10u8; 32]);

		// Signer 1
		let seckey1 = SecretKey::gen(&mut secp);
		let secnonce1 = SecretKey::gen(&mut secp);
		let pubnonce1 = PublicKey::from(&mut secp, &secnonce1)?;
		let pubkey1 = PublicKey::from(&mut secp, &seckey1)?;

		// Signer 2
		let seckey2 = SecretKey::gen(&mut secp);
		let secnonce2 = SecretKey::gen(&mut secp);
		let pubnonce2 = PublicKey::from(&mut secp, &secnonce2)?;
		let pubkey2 = PublicKey::from(&mut secp, &seckey2)?;

		// Sums
		let nonce_sum = pubnonce1.combine(&secp, &pubnonce2)?;
		let pubkey_sum = pubkey1.combine(&secp, &pubkey2)?;

		// Partial signatures with total sums
		let sig1 = secp.sign(&msg, &seckey1, &secnonce1, &nonce_sum, &pubkey_sum)?;

		assert!(secp
			.verify(&sig1, &msg, &nonce_sum, &pubkey1, &pubkey_sum, true)
			.is_ok());

		let sig2 = secp.sign(&msg, &seckey2, &secnonce2, &nonce_sum, &pubkey_sum)?;

		assert!(secp
			.verify(&sig2, &msg, &nonce_sum, &pubkey2, &pubkey_sum, true)
			.is_ok());

		// Aggregate
		let partial_sigs = &[&sig1, &sig2];
		let aggsig = secp.aggregate_signatures(partial_sigs, &nonce_sum)?;

		// Verify aggregated signature (non-zero-sum)
		assert!(secp
			.verify(&aggsig, &msg, &nonce_sum, &pubkey_sum, &pubkey_sum, false)
			.is_ok());

		let msgbad = Message::new([99u8; 32]);
		assert!(!secp
			.verify(
				&aggsig,
				&msgbad,
				&nonce_sum,
				&pubkey_sum,
				&pubkey_sum,
				false
			)
			.is_ok());

		Ok(())
	}

	#[test]
	fn test_balance_tx() -> Result<()> {
		let mut secp = Ctx::new()?;

		let blind1 = SecretKey::gen(&mut secp);
		let input1 = secp.commit(3000, &blind1)?;
		let blind2 = SecretKey::gen(&mut secp);
		let change = secp.commit(1000, &blind2)?;
		let blind3 = SecretKey::gen(&mut secp);
		let output1 = secp.commit(2000, &blind3)?;

		let excess_blind = secp.blind_sum(&[&blind1], &[&blind2, &blind3])?;
		let excess = secp.commit(0, &excess_blind)?;

		secp.verify_balance(&[&input1], &[&change, &output1, &excess], 0)?;

		let output1 = secp.commit(2001, &blind3)?;

		assert!(secp
			.verify_balance(&[&input1], &[&change, &output1, &excess], 0)
			.is_err());

		Ok(())
	}

	#[test]
	fn test_mimblewimble_interactive_with_fee() -> Result<()> {
		let mut secp_send = Ctx::new()?;
		let mut secp_recv = Ctx::new()?;
		let fee = 5u64;

		// Sender
		let blind1 = SecretKey::gen(&mut secp_send);
		let _input = secp_send.commit(500, &blind1)?;
		let blind2 = SecretKey::gen(&mut secp_send);
		let _change = secp_send.commit(100 - fee, &blind2)?;
		let sender_excess_blind = secp_send.blind_sum(&[&blind1], &[&blind2])?;
		let sender_excess = secp_send.commit(0, &sender_excess_blind)?;

		let sender_nonce = SecretKey::gen(&mut secp_send);
		let sender_pub_nonce = PublicKey::from(&secp_send, &sender_nonce)?;
		let sender_pubkey_sum = PublicKey::from(&secp_send, &sender_excess_blind)?;

		// Recipient
		let blind3 = SecretKey::gen(&mut secp_recv);
		let _output1 = secp_recv.commit(201, &blind3)?;
		let blind4 = SecretKey::gen(&mut secp_recv);
		let _output2 = secp_recv.commit(199, &blind4)?;
		let recipient_excess_blind = secp_recv.blind_sum(&[], &[&blind3, &blind4])?;
		let recipient_excess = secp_recv.commit(0, &recipient_excess_blind)?;

		let recipient_nonce = SecretKey::gen(&mut secp_recv);
		let recipient_pub_nonce = PublicKey::from(&secp_recv, &recipient_nonce)?;
		let recipient_pubkey_sum = PublicKey::from(&secp_recv, &recipient_excess_blind)?;

		let pub_nonce_sum = recipient_pub_nonce.combine(&mut secp_recv, &sender_pub_nonce)?;
		let pubkey_sum = recipient_pubkey_sum.combine(&secp_recv, &sender_pubkey_sum)?;
		let _excess = sender_excess.combine(&mut secp_recv, &recipient_excess)?;

		// simulation of message actually would hash the kernel and other data
		let msg = Message::new([1u8; 32]);

		// Recipient signs
		let sig_recv = secp_recv.sign(
			&msg,
			&recipient_excess_blind,
			&recipient_nonce,
			&pub_nonce_sum,
			&pubkey_sum,
		)?;
		assert!(secp_recv
			.verify(
				&sig_recv,
				&msg,
				&pub_nonce_sum,
				&recipient_pubkey_sum,
				&pubkey_sum,
				true
			)
			.is_ok());

		// Sender signs
		let sig_send = secp_send.sign(
			&msg,
			&sender_excess_blind,
			&sender_nonce,
			&pub_nonce_sum,
			&pubkey_sum,
		)?;
		assert!(secp_send
			.verify(
				&sig_send,
				&msg,
				&pub_nonce_sum,
				&sender_pubkey_sum,
				&pubkey_sum,
				true
			)
			.is_ok());

		// Aggregate and verify
		let partial_sigs = &[&sig_send, &sig_recv];
		let aggsig = secp_send.aggregate_signatures(partial_sigs, &pub_nonce_sum)?;
		assert!(secp_send
			.verify(
				&aggsig,
				&msg,
				&pub_nonce_sum,
				&pubkey_sum,
				&pubkey_sum,
				false
			)
			.is_ok());

		Ok(())
	}

	#[test]
	fn test_range_proofs() -> Result<()> {
		let mut ctx = Ctx::new()?;
		let blind = SecretKey::gen(&mut ctx);
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
