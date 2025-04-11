use crypto::ctx::Ctx;
use crypto::kernel::Kernel;
use crypto::keys::{Message, PublicKey, SecretKey, Signature};
use crypto::pedersen::Commitment;
use crypto::range_proof::RangeProof;
use mw::constants::KERNEL_FEATURE_PLAIN;
use mw::transaction::Transaction;
use prelude::*;

struct ParticipantData {
	inputs: Vec<Commitment>,
	outputs: Vec<(Commitment, RangeProof)>,
	pub_blind_excess: PublicKey,
	excess_commit: Commitment,
	pub_nonce: PublicKey,
	part_sig: Option<Signature>,
}

pub struct Slate {
	pdata: Vec<ParticipantData>,
	fee: u64,
}

impl Slate {
	pub fn new(fee: u64) -> Self {
		Self {
			pdata: Vec::new(),
			fee,
		}
	}

	pub fn commit(
		&mut self,
		ctx: &mut Ctx,
		input_keys: &[(&SecretKey, u64)],
		output_keys: &[(&SecretKey, u64)],
		sec_nonce: &SecretKey,
	) -> Result<usize, Error> {
		let mut inputs = Vec::with_capacity(input_keys.len())?;
		let mut outputs = Vec::with_capacity(output_keys.len())?;
		let mut input_keys_only = Vec::with_capacity(input_keys.len())?;
		let mut output_keys_only = Vec::with_capacity(output_keys.len())?;

		for input_key in input_keys {
			let commit = ctx.commit(input_key.1, input_key.0)?;
			inputs.push(commit)?;
			input_keys_only.push(input_key.0)?;
		}
		for output_key in output_keys {
			let commit = ctx.commit(output_key.1, output_key.0)?;
			let proof = ctx.range_proof(output_key.1, output_key.0)?;
			outputs.push((commit, proof))?;
			output_keys_only.push(output_key.0)?;
		}
		let pub_nonce = PublicKey::from(ctx, sec_nonce)?;

		let blind_excess = ctx.blind_sum(
			input_keys_only.slice(0, input_keys_only.len()),
			output_keys_only.slice(0, output_keys_only.len()),
		)?;
		let excess_commit = ctx.commit(0, &blind_excess)?;
		let pub_blind_excess = PublicKey::from(ctx, &blind_excess)?;

		let pd = ParticipantData {
			inputs,
			outputs,
			pub_nonce,
			pub_blind_excess,
			excess_commit,
			part_sig: None,
		};

		self.pdata.push(pd)?;
		Ok(self.pdata.len() - 1)
	}

	pub fn sign(
		&mut self,
		ctx: &mut Ctx,
		participant_id: usize,
		input_keys: &[&SecretKey],
		output_keys: &[&SecretKey],
		sec_nonce: &SecretKey,
	) -> Result<(), Error> {
		let excess_blind = ctx.blind_sum(input_keys, output_keys)?;
		let pub_nonce_sum = self.pub_nonce_sum(ctx)?;
		let pub_blind_sum = self.pub_blind_sum(ctx)?;
		let excess_commit = self.excess_commit_sum(ctx)?;
		let msg = ctx.hash_kernel(&excess_commit, self.fee, KERNEL_FEATURE_PLAIN)?;

		self.verify_part_sigs(ctx, participant_id, &msg, &pub_nonce_sum, &pub_blind_sum)?;

		let part_sig = ctx.sign_single(
			&msg,
			&excess_blind,
			sec_nonce,
			&pub_nonce_sum,
			&pub_blind_sum,
			&pub_nonce_sum,
		)?;

		self.pdata[participant_id].part_sig = Some(part_sig);

		Ok(())
	}

	pub fn finalize(&mut self, ctx: &mut Ctx) -> Result<Transaction, Error> {
		if self.pdata.len() == 0 {
			return Err(Error::new(IllegalState));
		}
		let excess_commit = self.excess_commit_sum(ctx)?;
		let pub_nonce_sum = self.pub_nonce_sum(ctx)?;

		let mut partial_sigs = Vec::with_capacity(self.pdata.len())?;
		for i in 0..self.pdata.len() {
			match &self.pdata[i].part_sig {
				Some(part_sig) => {
					partial_sigs.push(part_sig)?;
				}
				None => {
					return Err(Error::new(IllegalState));
				}
			}
		}
		let aggsig =
			ctx.aggregate_signatures(partial_sigs.slice(0, partial_sigs.len()), &pub_nonce_sum)?;
		let kernel = Kernel::new(excess_commit, aggsig, self.fee, 0);
		let mut tx = Transaction::new();
		tx.add_kernel(kernel)?;
		for i in 0..self.pdata.len() {
			for j in 0..self.pdata[i].inputs.len() {
				tx.add_input(self.pdata[i].inputs[j].clone())?;
			}
			for j in 0..self.pdata[i].outputs.len() {
				tx.add_output(
					self.pdata[i].outputs[j].0.clone(),
					self.pdata[i].outputs[j].1.clone(),
				)?;
			}
		}

		Ok(tx)
	}

	fn verify_part_sigs(
		&self,
		ctx: &mut Ctx,
		participant_id: usize,
		msg: &Message,
		pub_nonce_sum: &PublicKey,
		pub_blind_sum: &PublicKey,
	) -> Result<(), Error> {
		for i in 0..self.pdata.len() {
			if i != participant_id {
				match &self.pdata[i].part_sig {
					Some(part_sig) => {
						if !ctx.verify_single(
							part_sig,
							msg,
							pub_nonce_sum,
							&self.pdata[i].pub_blind_excess,
							pub_blind_sum,
							true,
						)? {
							return Err(Error::new(InvalidSignature));
						}
					}
					None => {}
				}
			}
		}
		Ok(())
	}

	fn excess_commit_sum(&self, ctx: &mut Ctx) -> Result<Commitment, Error> {
		if self.pdata.len() == 0 {
			return Err(Error::new(IllegalState));
		}
		let mut ret = self.pdata[0].excess_commit.clone();
		for i in 1..self.pdata.len() {
			ret = ret.add(ctx, &self.pdata[i].excess_commit.clone())?;
		}
		Ok(ret)
	}

	fn pub_nonce_sum(&self, ctx: &mut Ctx) -> Result<PublicKey, Error> {
		if self.pdata.len() == 0 {
			return Err(Error::new(IllegalState));
		}
		let mut ret = self.pdata[0].pub_nonce.clone();
		for i in 1..self.pdata.len() {
			ret = ret.add(ctx, &self.pdata[i].pub_nonce.clone())?;
		}
		Ok(ret)
	}

	fn pub_blind_sum(&self, ctx: &mut Ctx) -> Result<PublicKey, Error> {
		if self.pdata.len() == 0 {
			return Err(Error::new(IllegalState));
		}
		let mut ret = self.pdata[0].pub_blind_excess.clone();
		for i in 1..self.pdata.len() {
			ret = ret.add(ctx, &self.pdata[i].pub_blind_excess.clone())?;
		}
		Ok(ret)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use mw::keychain::KeyChain;

	#[test]
	fn test_slate1() -> Result<(), Error> {
		let mut ctx = Ctx::new()?;
		let mut slate = Slate::new(10);

		let user1_input_key = SecretKey::new(&mut ctx);
		let user1_change_key = SecretKey::new(&mut ctx);
		let user1_sec_nonce = SecretKey::new(&mut ctx);
		assert_eq!(
			slate.commit(
				&mut ctx,
				&[(&user1_input_key, 100)],
				&[(&user1_change_key, 10)],
				&user1_sec_nonce,
			)?,
			0
		);

		let user2_output_key = SecretKey::new(&mut ctx);
		let user2_sec_nonce = SecretKey::new(&mut ctx);

		assert_eq!(
			slate.commit(&mut ctx, &[], &[(&user2_output_key, 80)], &user2_sec_nonce)?,
			1
		);

		slate.sign(&mut ctx, 1, &[], &[&user2_output_key], &user2_sec_nonce)?;
		slate.sign(
			&mut ctx,
			0,
			&[&user1_input_key],
			&[&user1_change_key],
			&user1_sec_nonce,
		)?;
		let tx = slate.finalize(&mut ctx)?;
		tx.verify(&mut ctx, 1000)?;

		Ok(())
	}

	#[test]
	fn test_slate_with_keychain() -> Result<(), Error> {
		// create our crypto context
		let mut ctx = Ctx::new()?;

		// create a slate with a fee of 10 coins
		let mut slate = Slate::new(10);

		// user1 initiates request by offering to pay 100 coins with change of 10
		let kc1 = KeyChain::from_seed([0u8; 32])?;
		let user1_sec_nonce = SecretKey::new(&mut ctx); // random nonce
		let input = kc1.derive_key(&ctx, &[0, 0]); // choose a key for our input
		let change_output = kc1.derive_key(&ctx, &[0, 1]); // choose a key for the change

		// commit to the slate
		let user1_id = slate.commit(
			&mut ctx,
			&[(&input, 100)],
			&[(&change_output, 10)],
			&user1_sec_nonce,
		)?;

		// now it's user2's turn
		let kc2 = KeyChain::from_seed([1u8; 32])?;
		let user2_sec_nonce = SecretKey::new(&mut ctx); // random nonce
		let output = kc2.derive_key(&ctx, &[0, 0]); // choose an output

		// commit here we receive 80 coins (10 coin fee, 100 coin input and 10 coin change)
		let user2_id = slate.commit(&mut ctx, &[], &[(&output, 80)], &user2_sec_nonce)?;

		// now user2 signs the transaction
		slate.sign(&mut ctx, user2_id, &[], &[&output], &user2_sec_nonce)?;

		// now it's user1's turn to sign and finalize
		slate.sign(
			&mut ctx,
			user1_id,
			&[&input],
			&[&change_output],
			&user1_sec_nonce,
		)?;
		// finalize the slate
		let tx = slate.finalize(&mut ctx)?;
		// confirm the transaction is valid
		assert!(tx.verify(&mut ctx, 1000).is_ok());

		Ok(())
	}
}
