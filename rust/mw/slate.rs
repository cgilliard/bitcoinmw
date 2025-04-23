use crypto::{Commitment, Ctx, Message, PublicKey, RangeProof, SecretKey, Signature};
use mw::kernel::Kernel;
use mw::transaction::Transaction;
use prelude::*;

struct ParticipantData {
	inputs: Vec<Commitment>,
	outputs: Vec<(Commitment, RangeProof)>,
	pub_blind_excess: PublicKey,
	excess_commit: Commitment,
	pub_nonce: PublicKey,
	part_sig: Option<Signature>,
	sec_nonce: SecretKey,
}

pub struct Slate {
	pdata: Vec<ParticipantData>,
	fee: u64,
	offset: SecretKey,
}

impl Slate {
	pub fn new(fee: u64, offset: SecretKey) -> Self {
		Self {
			pdata: Vec::new(),
			fee,
			offset,
		}
	}

	pub fn commit(
		&mut self,
		ctx: &Ctx,
		input_keys: &[(&SecretKey, u64)],
		output_keys: &[(&SecretKey, u64)],
	) -> Result<usize, Error> {
		let mut inputs = Vec::with_capacity(input_keys.len())?;
		let mut outputs = Vec::with_capacity(output_keys.len())?;
		let mut input_keys_only = Vec::with_capacity(input_keys.len())?;
		let mut output_keys_only = Vec::with_capacity(output_keys.len())?;
		let sec_nonce = SecretKey::gen(&ctx);

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
		if self.pdata.len() == 0 {
			output_keys_only.push(&self.offset)?;
		}
		let pub_nonce = PublicKey::from(ctx, &sec_nonce)?;
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
			sec_nonce,
		};

		self.pdata.push(pd)?;
		Ok(self.pdata.len() - 1)
	}

	pub fn sign(
		&mut self,
		ctx: &Ctx,
		participant_id: usize,
		input_keys: &[&SecretKey],
		output_keys: &[&SecretKey],
	) -> Result<(), Error> {
		let mut output_keys_vec = Vec::new();
		for i in 0..output_keys.len() {
			output_keys_vec.push(output_keys[i])?;
		}
		if participant_id == 0 {
			output_keys_vec.push(&self.offset)?;
		}
		let excess_blind =
			ctx.blind_sum(input_keys, output_keys_vec.slice(0, output_keys_vec.len()))?;
		let pub_nonce_sum = self.pub_nonce_sum(ctx)?;
		let pub_blind_sum = self.pub_blind_sum(ctx)?;
		let excess_commit = self.excess_commit_sum(ctx)?;
		let msg = Kernel::message_for(&excess_commit, self.fee, 0);

		self.verify_part_sigs(ctx, participant_id, &msg, &pub_nonce_sum, &pub_blind_sum)?;
		let sec_nonce = &self.pdata[participant_id].sec_nonce;

		let part_sig = ctx.sign(
			&msg,
			&excess_blind,
			sec_nonce,
			&pub_nonce_sum,
			&pub_blind_sum,
		)?;

		self.pdata[participant_id].part_sig = Some(part_sig);

		// after signing we zero the sec_nonce to avoid exposing it later.
		self.pdata[participant_id].sec_nonce = SecretKey::zero();

		Ok(())
	}

	pub fn finalize(&mut self, ctx: &Ctx) -> Result<Transaction, Error> {
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
		let mut tx = Transaction::new(self.offset.clone());
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
		ctx: &Ctx,
		participant_id: usize,
		msg: &Message,
		pub_nonce_sum: &PublicKey,
		pub_blind_sum: &PublicKey,
	) -> Result<(), Error> {
		for i in 0..self.pdata.len() {
			if i != participant_id {
				match &self.pdata[i].part_sig {
					Some(part_sig) => {
						ctx.verify(
							part_sig,
							msg,
							pub_nonce_sum,
							&self.pdata[i].pub_blind_excess,
							pub_blind_sum,
							true,
						)?;
					}
					None => {}
				}
			}
		}
		Ok(())
	}

	fn excess_commit_sum(&self, ctx: &Ctx) -> Result<Commitment, Error> {
		// do we need to adjust commit sum for offset?
		if self.pdata.len() == 0 {
			return Err(Error::new(IllegalState));
		}
		let mut ret = self.pdata[0].excess_commit.clone();
		for i in 1..self.pdata.len() {
			ret = ret.combine(ctx, &self.pdata[i].excess_commit.clone())?;
		}
		Ok(ret)
	}

	fn pub_nonce_sum(&self, ctx: &Ctx) -> Result<PublicKey, Error> {
		if self.pdata.len() == 0 {
			return Err(Error::new(IllegalState));
		}
		let mut ret = self.pdata[0].pub_nonce.clone();
		for i in 1..self.pdata.len() {
			ret = ret.combine(ctx, &self.pdata[i].pub_nonce.clone())?;
		}
		Ok(ret)
	}

	fn pub_blind_sum(&self, ctx: &Ctx) -> Result<PublicKey, Error> {
		// do we need to adjust pub_blind_sum for offset?
		if self.pdata.len() == 0 {
			return Err(Error::new(IllegalState));
		}
		let mut ret = self.pdata[0].pub_blind_excess.clone();
		for i in 1..self.pdata.len() {
			ret = ret.combine(ctx, &self.pdata[i].pub_blind_excess.clone())?;
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
		let ctx = Ctx::new()?;
		let mut slate = Slate::new(10, SecretKey::gen(&ctx));

		let user1_input_key = SecretKey::gen(&ctx);
		let user1_change_key = SecretKey::gen(&ctx);
		assert_eq!(
			slate.commit(&ctx, &[(&user1_input_key, 100)], &[(&user1_change_key, 10)],)?,
			0
		);

		let user2_output_key = SecretKey::gen(&ctx);

		assert_eq!(slate.commit(&ctx, &[], &[(&user2_output_key, 80)])?, 1);

		slate.sign(&ctx, 1, &[], &[&user2_output_key])?;
		slate.sign(&ctx, 0, &[&user1_input_key], &[&user1_change_key])?;
		let tx = slate.finalize(&ctx)?;
		tx.validate(&ctx, 0)?;

		Ok(())
	}

	#[test]
	fn test_slate_with_keychain() -> Result<(), Error> {
		// create our crypto context
		let ctx = Ctx::new()?;

		// create a slate with a fee of 10 coins
		let mut slate = Slate::new(10, SecretKey::gen(&ctx));

		// user1 initiates request by offering to pay 100 coins with change of 10
		let kc1 = KeyChain::from_seed([0u8; 48])?;
		let input = kc1.derive_key(&ctx, &[0, 0]); // choose a key for our input
		let change_output = kc1.derive_key(&ctx, &[0, 1]); // choose a key for the change

		// commit to the slate
		let user1_id = slate.commit(&ctx, &[(&input, 100)], &[(&change_output, 10)])?;

		// now it's user2's turn
		let kc2 = KeyChain::from_seed([1u8; 48])?;
		let output = kc2.derive_key(&ctx, &[0, 0]); // choose an output

		// commit here we receive 80 coins (10 coin fee, 100 coin input and 10 coin change)
		let user2_id = slate.commit(&ctx, &[], &[(&output, 80)])?;

		// now user2 signs the transaction
		slate.sign(&ctx, user2_id, &[], &[&output])?;

		// now it's user1's turn to sign and finalize
		slate.sign(&ctx, user1_id, &[&input], &[&change_output])?;
		// finalize the slate
		let tx = slate.finalize(&ctx)?;
		// confirm the transaction is valid
		assert!(tx.validate(&ctx, 0).is_ok());

		Ok(())
	}

	#[test]
	fn test_merge() -> Result<(), Error> {
		// create our crypto context
		let ctx = Ctx::new()?;

		// create a slate with a fee of 10 coins
		let mut slate = Slate::new(10, SecretKey::gen(&ctx));

		// user1 initiates request by offering to pay 100 coins with change of 10
		let kc1 = KeyChain::from_seed([0u8; 48])?;
		let input = kc1.derive_key(&ctx, &[0, 0]); // choose a key for our input
		let change_output = kc1.derive_key(&ctx, &[0, 1]); // choose a key for the change

		// commit to the slate
		let user1_id = slate.commit(&ctx, &[(&input, 100)], &[(&change_output, 10)])?;

		// now it's user2's turn
		let kc2 = KeyChain::from_seed([1u8; 48])?;
		let output = kc2.derive_key(&ctx, &[0, 0]); // choose an output

		// commit here we receive 80 coins (10 coin fee, 100 coin input and 10 coin change)
		let user2_id = slate.commit(&ctx, &[], &[(&output, 80)])?;

		// now user2 signs the transaction
		slate.sign(&ctx, user2_id, &[], &[&output])?;

		// now it's user1's turn to sign and finalize
		slate.sign(&ctx, user1_id, &[&input], &[&change_output])?;
		// finalize the slate
		let tx = slate.finalize(&ctx)?;
		// confirm the transaction is valid
		assert!(tx.validate(&ctx, 0).is_ok());

		let mut slate = Slate::new(20, SecretKey::gen(&ctx));

		// user1 initiates request by offering to pay 100 coins with change of 10
		let kc1 = KeyChain::from_seed([2u8; 48])?;
		let input = kc1.derive_key(&ctx, &[0, 0]); // choose a key for our input
		let change_output = kc1.derive_key(&ctx, &[0, 1]); // choose a key for the change

		// commit to the slate
		let user1_id = slate.commit(&ctx, &[(&input, 200)], &[(&change_output, 30)])?;

		// now it's user2's turn
		let kc2 = KeyChain::from_seed([3u8; 48])?;
		let output = kc2.derive_key(&ctx, &[0, 0]); // choose an output

		// commit here we receive 80 coins (10 coin fee, 100 coin input and 10 coin change)
		let user2_id = slate.commit(&ctx, &[], &[(&output, 150)])?;

		// now user2 signs the transaction
		slate.sign(&ctx, user2_id, &[], &[&output])?;

		// now it's user1's turn to sign and finalize
		slate.sign(&ctx, user1_id, &[&input], &[&change_output])?;
		// finalize the slate
		let mut tx2 = slate.finalize(&ctx)?;
		// confirm the transaction is valid
		assert!(tx2.validate(&ctx, 0).is_ok());

		assert_eq!(tx.outputs().len(), 2);
		assert_eq!(tx2.outputs().len(), 2);
		assert_eq!(tx.inputs().len(), 1);
		assert_eq!(tx2.inputs().len(), 1);
		assert_eq!(tx.kernels().len(), 1);
		assert_eq!(tx2.kernels().len(), 1);

		tx2.merge(&ctx, tx)?;
		assert!(tx2.validate(&ctx, 0).is_ok());
		assert_eq!(tx2.outputs().len(), 4);
		assert_eq!(tx2.inputs().len(), 2);
		assert_eq!(tx2.kernels().len(), 2);

		// add coinbase
		let mut noffset = tx2.offset().unwrap().clone();
		noffset.negate(&ctx)?; // negate current offset to balance the block
		let mut coinbase = Transaction::new(noffset.clone());
		let kccb = KeyChain::from_seed([4u8; 48])?;
		let output_blind = kccb.derive_key(&ctx, &[0, 11]);

		// commit to 1000 + 30 in fees
		let cb_output = ctx.commit(1030, &output_blind)?;
		let cb_range_proof = ctx.range_proof(1030, &output_blind)?;
		let excess_blind = ctx.blind_sum(&[], &[&output_blind, &noffset])?;
		let excess = ctx.commit(0, &excess_blind)?;
		let msg = Kernel::message_for(&excess, 0, 0);
		let nonce = SecretKey::gen(&ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let pubkey = PublicKey::from(&ctx, &excess_blind)?;
		let sig = ctx.sign(&msg, &excess_blind, &nonce, &pubnonce, &pubkey)?;
		let kernel = Kernel::new(excess.clone(), sig.clone(), 0, 0);
		coinbase.add_kernel(kernel)?;
		coinbase.add_output(cb_output.clone(), cb_range_proof)?;
		assert!(coinbase.validate(&ctx, 1030).is_ok());

		tx2.merge(&ctx, coinbase.try_clone()?)?;
		assert_eq!(tx2.outputs().len(), 5);
		assert_eq!(tx2.inputs().len(), 2);
		assert_eq!(tx2.kernels().len(), 3);

		tx2.set_offset_zero();
		// verify with overage == 1000 (the block reward) Coinbase included 30 in fees from
		// merged transactions.
		assert!(tx2.validate(&ctx, 1000).is_ok());

		Ok(())
	}
}
