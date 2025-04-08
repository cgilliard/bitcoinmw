use core::ptr::{null, null_mut};
use crypto::constants::{GENERATOR_G, GENERATOR_H, SECP256K1_START_SIGN, SECP256K1_START_VERIFY};
use crypto::cpsrng::Cpsrng;
use crypto::ffi::*;
use crypto::keys::{PublicKey, PublicKeyUncompressed, SecretKey, Signature};
use crypto::pedersen::{Commitment, CommitmentUncompressed};
use crypto::types::Secp256k1Context;
use prelude::*;
use std::misc::to_le_bytes_u64;

pub struct Message([u8; 32]);

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
				retsig.0.as_mut_ptr(),
				msg.0.as_ptr(),
				seckey.0.as_ptr(),
				secnonce.0.as_ptr(),
				null(),
				pubnonce.0.as_ptr(),
				final_nonce_sum.0.as_ptr(),
				pe.0.as_ptr(),
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
				sig.0.as_ptr(),
				msg.0.as_ptr(),
				pubnonce.0.as_ptr(),
				pubkey.0.as_ptr(),
				pe.0.as_ptr(),
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
}

#[cfg(test)]
mod test {
	use super::*;

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
}
