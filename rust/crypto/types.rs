use constants::{
	GENERATOR_G, GENERATOR_H, SECP256K1_EC_COMPRESSED, SECP256K1_START_SIGN,
	SECP256K1_START_VERIFY, ZERO_KEY,
};
use core::mem::size_of;
use core::ptr::{copy_nonoverlapping, null, null_mut};
use core::sync::atomic::{compiler_fence, Ordering};
use ffi::*;
use prelude::*;
use std::misc::to_le_bytes_u64;

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

	pub fn hash_kernel(
		&self,
		excess: &Commitment,
		fee: u64,
		features: u64,
	) -> Result<Message, Error> {
		let mut sha3 = Sha3::new(Sha3_256)?;
		sha3.update(&excess.0); // 33 bytes

		let mut fee_bytes = [0u8; 8];
		to_le_bytes_u64(fee, &mut fee_bytes);
		sha3.update(&fee_bytes); // 8 bytes

		let mut features_bytes = [0u8; 8];
		to_le_bytes_u64(features, &mut features_bytes);
		sha3.update(&features_bytes); // 8 bytes

		let mut hash = [0u8; 32];
		sha3.finalize(&mut hash)?;
		Ok(Message(hash))
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

	pub fn sign_ecdsa(&self, msg: &[u8; 32], seckey: &SecretKey) -> Result<SignatureScalar, Error> {
		let mut sig = [0u8; 64];
		unsafe {
			if secp256k1_ecdsa_sign(
				self.ctx,
				sig.as_mut_ptr() as *mut Signature,
				msg.as_ptr(),
				seckey.0.as_ptr() as *const SecretKey,
				secp256k1_nonce_function_rfc6979,
				null_mut(),
			) != 1
			{
				return Err(Error::new(Secp));
			}
		}
		SignatureScalar::from_bytes(&sig[32..])
	}

	pub fn sign_single(
		&self,
		msg: &Message,
		seckey: &SecretKey,
		secnonce: &SecretKey,
		pubnonce: &PublicKey,
		pe: &PublicKey,
		final_nonce_sum: &PublicKey,
	) -> Result<Signature, Error> {
		let pubnonce_uncomp = match pubnonce.decompress(self) {
			Ok(p) => p,
			Err(e) => return Err(e),
		};

		let pe_uncomp = match pe.decompress(self) {
			Ok(p) => p,
			Err(e) => return Err(e),
		};

		let final_nonce_sum_uncomp = match final_nonce_sum.decompress(self) {
			Ok(f) => f,
			Err(e) => return Err(e),
		};
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
		&self,
		msg: &Message,
		seckey: &SecretKey,
		secnonce: &SecretKey,
		pubnonce: &PublicKeyUncompressed,
		pe: &PublicKeyUncompressed,
		final_nonce_sum: &PublicKeyUncompressed,
	) -> Result<Signature, Error> {
		let mut retsig = Signature([0u8; 64]);
		let mut seed = [0u8; 32];
		unsafe {
			cpsrng_rand_bytes(self.rand, seed.as_mut_ptr(), 32);
		}

		let retval = unsafe {
			secp256k1_aggsig_sign_single(
				self.ctx,
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
		let pubnonce_uncomp = match pubnonce.decompress(self) {
			Ok(p) => p,
			Err(e) => return Err(e),
		};

		let pe_uncomp = match pe.decompress(self) {
			Ok(p) => p,
			Err(e) => return Err(e),
		};

		let pubkey_uncomp = match pubkey.decompress(self) {
			Ok(p) => p,
			Err(e) => return Err(e),
		};
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
				self.ctx,
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

	pub fn aggregate_signatures(
		&self,
		partial_sigs: &[&Signature],
		nonce_sum: &PublicKey,
	) -> Result<Signature, Error> {
		let nonce_sum = match nonce_sum.decompress(self) {
			Ok(ns) => ns,
			Err(e) => return Err(e),
		};
		let mut sig = [0u8; 64];
		let num_sigs = partial_sigs.len();
		if num_sigs > 16 {
			return Err(Error::new(TooManySignatures));
		}
		let mut sig_ptrs = [null(); 16];
		for i in 0..num_sigs {
			sig_ptrs[i] = &partial_sigs[i].0 as *const u8;
		}
		unsafe {
			if secp256k1_aggsig_add_signatures_single(
				self.ctx,
				sig.as_mut_ptr(),
				sig_ptrs.as_ptr(),
				num_sigs,
				nonce_sum.0.as_ptr(),
			) != 1
			{
				return Err(Error::new(InvalidSignature));
			}
		}
		Ok(Signature(sig))
	}

	pub fn pubkey_uncompressed(&self, skey: &SecretKey) -> Result<PublicKeyUncompressed, Error> {
		let mut uncomp = [0u8; 64];
		unsafe {
			if secp256k1_ec_pubkey_create(
				self.ctx,
				uncomp.as_mut_ptr() as *mut PublicKeyUncompressed,
				skey.0.as_ptr() as *const SecretKey,
			) != 1
			{
				return Err(Error::new(InvalidPublicKey));
			}
		}
		Ok(PublicKeyUncompressed(uncomp))
	}

	pub fn sign_partial(
		&self,
		msg: &Message, // 32-byte SHA-3 hash
		inputs: &[&SecretKey],
		outputs: &[&SecretKey],
		sec_nonce: &SecretKey, // Participant’s nonce
		nonce_sum: &PublicKey, // Total nonce sum
		pubkey_sum: &PublicKey,
	) -> Result<Signature, Error> {
		let mut sig = [0u8; 64];
		let mut seed = [0u8; 32];
		unsafe {
			cpsrng_rand_bytes(self.rand, seed.as_mut_ptr(), 32);
		}

		let seckey = match self.blind_sum(inputs, outputs) {
			Ok(s) => s,
			Err(e) => return Err(e),
		};

		unsafe {
			if secp256k1_aggsig_sign_single(
				self.ctx,
				sig.as_mut_ptr(),
				msg.0.as_ptr(),
				seckey.0.as_ptr(),
				sec_nonce.0.as_ptr(),
				null(),
				nonce_sum.0.as_ptr(),
				nonce_sum.0.as_ptr(),
				pubkey_sum.0.as_ptr(),
				seed.as_ptr(),
			) != 1
			{
				return Err(Error::new(InvalidSignature));
			}
		}
		Ok(Signature(sig))
	}

	pub fn verify_partial(
		&self,
		msg: &Message, // 32-byte SHA-3 hash
		sig: &Signature,
		pubkey: &PublicKey,     // Participant’s blinding sum
		nonce: &PublicKey,      // Participant’s nonce
		pubkey_sum: &PublicKey, // Total blinding sum
	) -> Result<(), Error> {
		unsafe {
			if secp256k1_aggsig_verify_single(
				self.ctx,
				sig.0.as_ptr(),
				msg.0.as_ptr(),
				nonce.0.as_ptr(),
				pubkey.0.as_ptr(),
				pubkey_sum.0.as_ptr(),
				null(),
				1, // is_partial = true
			) != 1
			{
				return Err(Error::new(IncorrectSignature));
			}
		}
		Ok(())
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

	pub fn verify_kernel(
		&self,
		msg: &Message,
		sig: &Signature,
		excess: &Commitment,
	) -> Result<(), Error> {
		println!("verify kernel");
		let mut pubkey = [0u8; 33];
		unsafe {
			if secp256k1_pedersen_commitment_to_pubkey(
				self.ctx,
				pubkey.as_mut_ptr(),
				excess.0.as_ptr(),
			) != 1
			{
				println!("Invalid commitment");
				return Err(Error::new(InvalidCommitment));
			}

			if secp256k1_aggsig_verify_single(
				self.ctx,
				sig.0.as_ptr(),
				msg.0.as_ptr(),
				null(), // pubnonce (null for completed sig)
				pubkey.as_ptr(),
				null(), // pk_total (null for final)
				null(), // extra_pubkey
				0,      // is_partial = false
			) != 1
			{
				println!("Incorrect signature");
				return Err(Error::new(IncorrectSignature));
			}
		}
		println!("ok");
		Ok(())
	}

	pub fn verify_balance(
		&self,
		positive: &[&Commitment],
		negative: &[&Commitment],
		fee: u64,
	) -> Result<bool, Error> {
		// convert slice of positive/negative commitments to vecs of uncompressed
		// commitments
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

		// handle fee
		if fee != 0 {
			let rblind = SecretKey::new(self);
			let p = self.commit(0, &rblind)?;
			let n = self.commit(fee, &rblind)?;
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
				self.ctx,
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
}

impl SignatureScalar {
	pub fn zero() -> Self {
		SignatureScalar([0u8; 32])
	}

	pub fn from_bytes(b: &[u8]) -> Result<Self, Error> {
		if b.len() != 32 {
			Err(Error::new(IllegalArgument))
		} else {
			let mut arr = [0u8; 32];
			unsafe {
				copy_nonoverlapping(b.as_ptr(), arr.as_mut_ptr(), b.len());
			}
			Ok(SignatureScalar(arr))
		}
	}

	pub fn add(&mut self, secp: &Secp, other: &SignatureScalar) -> Result<(), Error> {
		unsafe {
			if secp256k1_ec_privkey_tweak_add(
				secp.ctx,
				self.0.as_mut_ptr() as *mut SecretKey,
				other.0.as_ptr() as *const SecretKey,
			) != 1
			{
				return Err(Error::new(InvalidScalar));
			}
		}
		Ok(())
	}

	pub fn sub(&mut self, secp: &Secp, other: &SignatureScalar) -> Result<(), Error> {
		let mut neg = other.clone();
		unsafe {
			if secp256k1_ec_privkey_negate(secp.ctx, neg.0.as_mut_ptr() as *mut SecretKey) != 1
				|| secp256k1_ec_privkey_tweak_add(
					secp.ctx,
					self.0.as_mut_ptr() as *mut SecretKey,
					neg.0.as_ptr() as *const SecretKey,
				) != 1
			{
				return Err(Error::new(InvalidScalar));
			}
		}
		Ok(())
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
	pub fn decompress(&self, secp: &Secp) -> Result<CommitmentUncompressed, Error> {
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

	pub fn compress(secp: &Secp, key: CommitmentUncompressed) -> Result<Self, Error> {
		let mut v = [0u8; 33];
		let serialize_result = unsafe {
			secp256k1_pedersen_commitment_serialize(secp.ctx, v.as_mut_ptr(), &key.0 as *const u8)
		};
		if serialize_result == 0 {
			Err(Error::new(Serialization))
		} else {
			Ok(Self(v))
		}
	}

	pub fn to_pubkey(&self, secp: &Secp) -> Result<PublicKey, Error> {
		let mut pk = [0u8; 64];
		unsafe {
			let commit = self.0;
			if secp256k1_pedersen_commitment_to_pubkey(
				secp.ctx,
				pk.as_mut_ptr() as *mut _,
				commit.as_ptr(),
			) == 1
			{
				match PublicKey::compress(&secp, PublicKeyUncompressed(pk)) {
					Ok(pk) => Ok(pk),
					Err(e) => Err(e),
				}
			} else {
				Err(Error::new(InvalidPublicKey))
			}
		}
	}

	pub fn add(&self, secp: &Secp, other: &Commitment) -> Result<Commitment, Error> {
		let pk1 = match self.to_pubkey(secp) {
			Ok(pk1) => pk1,
			Err(e) => return Err(e),
		};
		let pk2 = match other.to_pubkey(secp) {
			Ok(pk2) => pk2,
			Err(e) => return Err(e),
		};
		match pk1.add(secp, &pk2) {
			Ok(sum_pk) => Ok(Commitment(sum_pk.0)),
			Err(e) => return Err(e),
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

	pub fn compress(secp: &Secp, key: PublicKeyUncompressed) -> Result<Self, Error> {
		let mut v = [0u8; 33];
		let mut len = 33usize;
		let serialize_result = unsafe {
			secp256k1_ec_pubkey_serialize(
				secp.ctx,
				v.as_mut_ptr(),
				&mut len,
				&key.0 as *const u8 as *const PublicKeyUncompressed,
				SECP256K1_EC_COMPRESSED,
			)
		};
		if serialize_result == 0 {
			Err(Error::new(Serialization))
		} else {
			Ok(Self(v))
		}
	}

	pub fn decompress(&self, secp: &Secp) -> Result<PublicKeyUncompressed, Error> {
		let mut ret = [0u8; 64];
		unsafe {
			if secp256k1_ec_pubkey_parse(
				secp.ctx,
				ret.as_mut_ptr() as *mut PublicKeyUncompressed,
				self.0.as_ptr(),
				self.0.len(),
			) != 1
			{
				return Err(Error::new(InvalidPublicKey));
			}
		}
		Ok(PublicKeyUncompressed(ret))
	}

	pub fn add(&self, secp: &Secp, other: &PublicKey) -> Result<Self, Error> {
		let mut result = [0u8; 64];
		let mut uncomp_self = [0u8; 64];
		let mut uncomp_other = [0u8; 64];

		// Uncompress self
		unsafe {
			if secp256k1_ec_pubkey_parse(
				secp.ctx,
				uncomp_self.as_mut_ptr() as *mut PublicKeyUncompressed,
				self.0.as_ptr(),
				self.0.len(),
			) != 1
			{
				return Err(Error::new(InvalidPublicKey));
			}

			// Uncompress other
			if secp256k1_ec_pubkey_parse(
				secp.ctx,
				uncomp_other.as_mut_ptr() as *mut PublicKeyUncompressed,
				other.0.as_ptr(),
				other.0.len(),
			) != 1
			{
				return Err(Error::new(InvalidPublicKey));
			}

			// Combine uncompressed keys
			let pubkeys = [uncomp_self.as_ptr(), uncomp_other.as_ptr()];
			if secp256k1_ec_pubkey_combine(secp.ctx, result.as_mut_ptr(), pubkeys.as_ptr(), 2) != 1
			{
				return Err(Error::new(InvalidPublicKey));
			}

			// Recompress result
			let mut compressed = [0u8; 33];
			let mut len = 33usize;
			if secp256k1_ec_pubkey_serialize(
				secp.ctx,
				compressed.as_mut_ptr(),
				&mut len,
				result.as_ptr() as *const PublicKeyUncompressed,
				SECP256K1_EC_COMPRESSED,
			) != 1
			{
				return Err(Error::new(Serialization));
			}
			Ok(Self(compressed))
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
		let blind4 = secp.blind_sum(&[&blind1, &blind2], &[&blind3]).unwrap();
		// create an output with this balancing factor and amount
		let output2 = secp.commit(2000, &blind4).unwrap();

		// verify balance
		assert!(secp
			.verify_balance(&[&input1, &input2], &[&output1, &output2], 0)
			.unwrap());

		// negative test
		let blind1 = SecretKey::new(&secp);
		let blind2 = SecretKey::new(&secp);
		let blind3 = SecretKey::new(&secp);
		let input1 = secp.commit(1000, &blind1).unwrap();
		let input2 = secp.commit(3000, &blind2).unwrap();
		let output1 = secp.commit(2000, &blind3).unwrap();
		let blind4 = secp.blind_sum(&[&blind1, &blind2], &[&blind3]).unwrap();
		let output_bad = secp.commit(2001, &blind4).unwrap();
		assert!(!secp
			.verify_balance(&[&input1, &input2], &[&output1, &output_bad], 0)
			.unwrap());

		assert!(
			secp.verify_balance(&[], &[], 0).unwrap(),
			"Empty commitments should balance"
		);
		let zero_commit = secp.commit(0, &blind1).unwrap();
		assert!(secp
			.verify_balance(&[&zero_commit], &[&zero_commit], 0)
			.unwrap());
	}

	#[test]
	fn test_simple_sign() {
		let secp = Secp::new().unwrap();
		let seckey = SecretKey::new(&secp);
		let secnonce = SecretKey::new(&secp);
		let pubnonce = PublicKey::from(&secp, &secnonce).unwrap();
		let pubkey = PublicKey::from(&secp, &seckey).unwrap();

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
		let secp = Secp::new().unwrap();
		let msg = Message([10u8; 32]);

		// Signer 1
		let seckey1 = SecretKey::new(&secp);
		let secnonce1 = SecretKey::new(&secp);
		let pubnonce1 = PublicKey::from(&secp, &secnonce1).unwrap();
		let pubkey1 = PublicKey::from(&secp, &seckey1).unwrap();

		// Signer 2
		let seckey2 = SecretKey::new(&secp);
		let secnonce2 = SecretKey::new(&secp);
		let pubnonce2 = PublicKey::from(&secp, &secnonce2).unwrap();
		let pubkey2 = PublicKey::from(&secp, &seckey2).unwrap();

		// Sums
		let nonce_sum = pubnonce1.add(&secp, &pubnonce2).unwrap();
		//let nonce_sum = nonce_sum_comp.decompress(&secp).unwrap();
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
		let secp = Secp::new().unwrap();

		let blind1 = SecretKey::new(&secp);
		let input1 = secp.commit(3000, &blind1).unwrap();
		let blind2 = SecretKey::new(&secp);
		let change = secp.commit(1000, &blind2).unwrap();
		let blind3 = SecretKey::new(&secp);
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
		let secp = Secp::new().unwrap();

		let blind1 = SecretKey::new(&secp);
		let input1 = secp.commit(3000, &blind1).unwrap();
		let blind2 = SecretKey::new(&secp);
		let change = secp.commit(1000, &blind2).unwrap();
		let blind3 = SecretKey::new(&secp);
		let output1 = secp.commit(2000, &blind3).unwrap();

		let excess_blind = secp.blind_sum(&[&blind1], &[&blind2, &blind3]).unwrap();
		let excess = secp.commit(0, &excess_blind).unwrap();
		let msg = secp.hash_kernel(&excess, 0, 0).unwrap();

		let pubkey_sum = PublicKey::from(&secp, &excess_blind).unwrap();

		let nonce1 = SecretKey::new(&secp);
		let pubnonce1 = PublicKey::from(&secp, &nonce1).unwrap();
		let sender_blind = secp.blind_sum(&[&blind1], &[&blind2]).unwrap();
		let pub_sender = PublicKey::from(&secp, &sender_blind).unwrap();

		let nonce2 = SecretKey::new(&secp);
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
		let secp_send = Secp::new().unwrap();
		let secp_recv = Secp::new().unwrap();

		// start with sender
		let blind1 = SecretKey::new(&secp_send);
		let input = secp_send.commit(500, &blind1).unwrap();
		let blind2 = SecretKey::new(&secp_send);
		let change = secp_send.commit(100, &blind2).unwrap();
		let sender_excess_blind = secp_send.blind_sum(&[&blind1], &[&blind2]).unwrap();
		let sender_excess = secp_send.commit(0, &sender_excess_blind).unwrap();

		// sender generates private nonce and calcualates other values
		let sender_nonce = SecretKey::new(&secp_send);
		let sender_pub_nonce = PublicKey::from(&secp_send, &sender_nonce).unwrap();
		let sender_pubkey_sum = PublicKey::from(&secp_send, &sender_excess_blind).unwrap();

		// SLATE SEND 1

		// now recipient gets slate and adds outputs
		let blind3 = SecretKey::new(&secp_recv);
		let output1 = secp_recv.commit(201, &blind3).unwrap();
		let blind4 = SecretKey::new(&secp_recv);
		let output2 = secp_recv.commit(199, &blind4).unwrap();
		let recipient_excess_blind = secp_recv.blind_sum(&[], &[&blind3, &blind4]).unwrap();
		let recipient_excess = secp_recv.commit(0, &recipient_excess_blind).unwrap();

		// recipient generates private nonce
		let recipient_nonce = SecretKey::new(&secp_recv);
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
		let secp_send = Secp::new()?;
		let secp_recv = Secp::new()?;

		let fee = 5u64;

		// Sender
		let blind1 = SecretKey::new(&secp_send);
		let input = secp_send.commit(500, &blind1)?;
		let blind2 = SecretKey::new(&secp_send);
		let change = secp_send.commit(100 - fee, &blind2)?; // Fee deducted from change (95)
		let sender_excess_blind = secp_send.blind_sum(&[&blind1], &[&blind2])?;
		let sender_excess = secp_send.commit(0, &sender_excess_blind)?; // Excess remains 0*H

		let sender_nonce = SecretKey::new(&secp_send);
		let sender_pub_nonce = PublicKey::from(&secp_send, &sender_nonce)?;
		let sender_pubkey_sum = PublicKey::from(&secp_send, &sender_excess_blind)?;

		// Recipient
		let blind3 = SecretKey::new(&secp_recv);
		let output1 = secp_recv.commit(201, &blind3)?;
		let blind4 = SecretKey::new(&secp_recv);
		let output2 = secp_recv.commit(199, &blind4)?; // Full amount, no fee here
		let recipient_excess_blind = secp_recv.blind_sum(&[], &[&blind3, &blind4])?;
		let recipient_excess = secp_recv.commit(0, &recipient_excess_blind)?;

		let recipient_nonce = SecretKey::new(&secp_recv);
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
}
