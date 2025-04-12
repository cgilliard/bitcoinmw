use crypto::{Commitment, Ctx, RangeProof, SecretKey};
use mw::kernel::Kernel;
use prelude::*;

pub struct Transaction {
	inputs: Vec<Commitment>,
	outputs: Vec<(Commitment, RangeProof)>,
	kernels: Vec<Kernel>,
	offset: Option<SecretKey>,
}

impl TryClone for Transaction {
	fn try_clone(&self) -> Result<Self, Error>
	where
		Self: Sized,
	{
		Ok(Self {
			inputs: self.inputs.try_clone()?,
			outputs: self.outputs.try_clone()?,
			kernels: self.kernels.try_clone()?,
			offset: self.offset.clone(),
		})
	}
}

impl Transaction {
	pub fn new(skey: SecretKey) -> Self {
		Self {
			inputs: Vec::new(),
			outputs: Vec::new(),
			kernels: Vec::new(),
			offset: Some(skey),
		}
	}

	pub fn merge(&mut self, ctx: &Ctx, t: Transaction) -> Result<(), Error> {
		let mut kernels = self.kernels.try_clone()?;
		let mut inputs = self.inputs.try_clone()?;
		let mut outputs = self.outputs.try_clone()?;

		kernels.append(&t.kernels)?;
		inputs.append(&t.inputs)?;
		outputs.append(&t.outputs)?;

		match &self.offset {
			Some(self_offset) => match &t.offset {
				Some(t_offset) => {
					let offset = ctx.blind_sum(&[&self_offset, &t_offset], &[])?;
					self.offset = Some(offset);
				}
				None => {}
			},
			None => match &t.offset {
				Some(t_offset) => self.offset = Some(t_offset.clone()),
				None => {}
			},
		}
		self.kernels = kernels;
		self.inputs = inputs;
		self.outputs = outputs;

		Ok(())
	}

	pub fn add_input(&mut self, input: Commitment) -> Result<(), Error> {
		self.inputs.push(input)
	}

	pub fn add_output(&mut self, output: Commitment, proof: RangeProof) -> Result<(), Error> {
		self.outputs.push((output, proof))
	}

	pub fn add_kernel(&mut self, kernel: Kernel) -> Result<(), Error> {
		self.kernels.push(kernel)
	}

	pub fn verify(&self, ctx: &mut Ctx, overage: u64) -> Result<(), Error> {
		let offset_commit: Commitment;
		{
			if self.kernels.len() == 0 {
				return Err(Error::new(KernelNotFound));
			}
			if self.outputs.len() == 0 {
				return Err(Error::new(InvalidTransaction));
			}

			for i in 0..self.outputs.len() {
				let output = &self.outputs[i];
				ctx.verify_range_proof(&output.0, &output.1)?;
			}
			let mut input_commits: Vec<&Commitment> = Vec::new();
			for i in 0..self.inputs.len() {
				input_commits.push(&self.inputs[i])?;
			}

			let mut output_commits: Vec<&Commitment> = Vec::new();
			for i in 0..self.outputs.len() {
				output_commits.push(&self.outputs[i].0)?;
			}

			if let Some(offset) = &self.offset {
				offset_commit = ctx.commit(0, offset)?;
				output_commits.push(&offset_commit)?;
			}

			let mut fee = 0;
			for i in 0..self.kernels.len() {
				output_commits.push(self.kernels[i].excess())?;
				let msg = ctx.hash_kernel(
					self.kernels[i].excess(),
					self.kernels[i].fee(),
					self.kernels[i].features(),
				)?;
				self.kernels[i].verify(ctx, &msg)?;
				fee += self.kernels[i].fee();
			}

			let inp = if input_commits.len() == 0 {
				&[]
			} else {
				input_commits.slice(0, input_commits.len())
			};

			ctx.verify_balance(
				inp,
				output_commits.slice(0, output_commits.len()),
				(fee - overage) as i128,
			)?;
		}

		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crypto::{Ctx, PublicKey, SecretKey};
	use mw::keychain::KeyChain;

	#[test]
	fn test_transaction() -> Result<(), Error> {
		let mut ctx = Ctx::new()?;
		let offset = SecretKey::gen(&ctx);
		let blind1 = SecretKey::gen(&ctx);
		let input = ctx.commit(500, &blind1)?;
		let blind2 = SecretKey::gen(&ctx);
		let blind3 = SecretKey::gen(&ctx);
		let blind4 = SecretKey::gen(&ctx);
		let output1 = ctx.commit(100, &blind2)?;
		let output2 = ctx.commit(100, &blind3)?;
		let output3 = ctx.commit(250, &blind4)?;
		let rp1 = ctx.range_proof(100, &blind2)?;
		let rp2 = ctx.range_proof(100, &blind3)?;
		let rp3 = ctx.range_proof(250, &blind4)?;
		let fee = 50;

		let excess_blind = ctx.blind_sum(&[&blind1], &[&blind2, &blind3, &blind4, &offset])?;
		let excess = ctx.commit(0, &excess_blind)?;
		let msg = ctx.hash_kernel(&excess, fee, 0)?;

		let nonce = SecretKey::gen(&mut ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let pubkey = PublicKey::from(&ctx, &excess_blind)?;

		let sig = ctx.sign_single(&msg, &excess_blind, &nonce, &pubnonce, &pubkey)?;
		let kernel = Kernel::new(excess.clone(), sig.clone(), fee, 0);

		let mut tx = Transaction::new(offset.clone());
		tx.add_input(input.clone())?;
		tx.add_output(output1.clone(), rp1.clone())?;
		tx.add_output(output2.clone(), rp2.clone())?;
		tx.add_output(output3.clone(), rp3.clone())?;
		tx.add_kernel(kernel)?;
		assert!(tx.verify(&mut ctx, 0).is_ok());

		let mut tx = Transaction::new(offset);
		let kernel = Kernel::new(excess, sig, fee + 1, 0);
		tx.add_input(input)?;
		tx.add_output(output1, rp1)?;
		tx.add_output(output2, rp2)?;
		tx.add_output(output3, rp3)?;
		tx.add_kernel(kernel)?;
		assert!(tx.verify(&mut ctx, 0).is_err());

		Ok(())
	}

	fn test_interactive() -> Result<(), Error> {
		let ctx_user1 = Ctx::new()?;
		let offset = SecretKey::gen(&ctx_user1);
		let blind1 = SecretKey::gen(&ctx_user1);
		let blind2 = SecretKey::gen(&ctx_user1);
		let input = ctx_user1.commit(5_000, &blind1)?;
		let change_output = ctx_user1.commit(1_000, &blind1)?;
		let change_range_proof = ctx_user1.range_proof(1_000, &blind1)?;
		let nonce_user1 = SecretKey::gen(&ctx_user1);
		let pub_nonce_user1 = PublicKey::from(&ctx_user1, &nonce_user1)?;
		let excess_blind_user1 = ctx_user1.blind_sum(&[&blind1], &[&blind2, &offset])?;
		let pub_blind_user1 = PublicKey::from(&ctx_user1, &excess_blind_user1)?;
		let excess_user1 = ctx_user1.commit(0, &excess_blind_user1)?;
		let fee = 50;

		let mut ctx_user2 = Ctx::new()?;
		let blind3 = SecretKey::gen(&ctx_user2);
		let output = ctx_user2.commit(3_950, &blind3)?;
		let range_proof = ctx_user2.range_proof(3_950, &blind3)?;
		let nonce_user2 = SecretKey::gen(&ctx_user2);
		let pub_nonce_user2 = PublicKey::from(&ctx_user2, &nonce_user2)?;
		let excess_blind_user2 = ctx_user2.blind_sum(&[], &[&blind3])?;
		let pub_blind_user2 = PublicKey::from(&ctx_user2, &excess_blind_user2)?;
		let excess_user2 = ctx_user2.commit(0, &excess_blind_user2)?;

		let pubnonce_sum = pub_nonce_user1.add(&ctx_user2, &pub_nonce_user2)?;
		let pubblind_sum = pub_blind_user1.add(&ctx_user2, &pub_blind_user2)?;
		let excess_sum = excess_user2.add(&ctx_user2, &excess_user1)?;
		let msg = ctx_user2.hash_kernel(&excess_sum, fee, 0)?;

		let sig2 = ctx_user2.sign_single(
			&msg,
			&excess_blind_user2,
			&nonce_user2,
			&pubnonce_sum,
			&pubblind_sum,
		)?;

		let sig1 = ctx_user1.sign_single(
			&msg,
			&excess_blind_user1,
			&nonce_user1,
			&pubnonce_sum,
			&pubblind_sum,
		)?;

		let partial_sigs = vec![&sig1, &sig2]?;
		let aggsig = ctx_user1
			.aggregate_signatures(partial_sigs.slice(0, partial_sigs.len()), &pubnonce_sum)?;

		let kernel = Kernel::new(excess_sum, aggsig, fee, 0);
		let mut tx = Transaction::new(offset);
		tx.add_kernel(kernel)?;
		tx.add_output(change_output, change_range_proof)?;
		tx.add_output(output, range_proof)?;
		tx.add_input(input)?;
		assert!(tx.verify(&mut ctx_user2, 0).is_ok());

		Ok(())
	}

	/*
	#[test]
	fn test_transaction() -> Result<(), Error> {
		let mut ctx = Ctx::new()?;
		let blind1 = SecretKey::new(&mut ctx);
		let input = ctx.commit(500, &blind1)?;
		let blind2 = SecretKey::new(&mut ctx);
		let blind3 = SecretKey::new(&mut ctx);
		let blind4 = SecretKey::new(&mut ctx);
		let output1 = ctx.commit(100, &blind2)?;
		let output2 = ctx.commit(100, &blind3)?;
		let output3 = ctx.commit(250, &blind4)?;
		let rp1 = ctx.range_proof(100, &blind2)?;
		let rp2 = ctx.range_proof(100, &blind3)?;
		let rp3 = ctx.range_proof(250, &blind4)?;
		let fee = 50;

		let excess_blind = ctx.blind_sum(&[&blind1], &[&blind2, &blind3, &blind4])?;
		let excess = ctx.commit(0, &excess_blind)?;
		let msg = ctx.hash_kernel(&excess, fee, 0)?;

		let nonce = SecretKey::new(&mut ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let pubkey = PublicKey::from(&ctx, &excess_blind)?;

		let sig = ctx.sign_single(&msg, &excess_blind, &nonce, &pubnonce, &pubkey, &pubnonce)?;
		let kernel = Kernel::new(excess.clone(), sig.clone(), fee, 0);

		let mut tx = Transaction::new();
		tx.add_input(input.clone())?;
		tx.add_output(output1.clone(), rp1.clone())?;
		tx.add_output(output2.clone(), rp2.clone())?;
		tx.add_output(output3.clone(), rp3.clone())?;
		tx.add_kernel(kernel)?;
		assert!(tx.verify(&mut ctx, 1000).is_ok());

		let mut tx = Transaction::new();
		let kernel = Kernel::new(excess, sig, fee + 1, 0);
		tx.add_input(input)?;
		tx.add_output(output1, rp1)?;
		tx.add_output(output2, rp2)?;
		tx.add_output(output3, rp3)?;
		tx.add_kernel(kernel)?;
		assert!(tx.verify(&mut ctx, 1000).is_err());

		Ok(())
	}

	#[test]
	fn test_transaction_with_offset() -> Result<(), Error> {
		let mut ctx = Ctx::new()?;
		let offset = SecretKey::new(&mut ctx);
		let blind1 = SecretKey::new(&mut ctx);
		let input = ctx.commit(500, &blind1)?;
		let blind2 = SecretKey::new(&mut ctx);
		let blind3 = SecretKey::new(&mut ctx);
		let blind4 = SecretKey::new(&mut ctx);
		let output1 = ctx.commit(100, &blind2)?;
		let output2 = ctx.commit(100, &blind3)?;
		let output3 = ctx.commit(250, &blind4)?;
		let rp1 = ctx.range_proof(100, &blind2)?;
		let rp2 = ctx.range_proof(100, &blind3)?;
		let rp3 = ctx.range_proof(250, &blind4)?;
		let fee = 50;

		let excess_blind = ctx.blind_sum(&[&blind1], &[&blind2, &blind3, &blind4, &offset])?;
		let excess = ctx.commit(0, &excess_blind)?;
		let msg = ctx.hash_kernel(&excess, fee, 0)?;

		let nonce = SecretKey::new(&mut ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let pubkey = PublicKey::from(&ctx, &excess_blind)?;

		let sig = ctx.sign_single(&msg, &excess_blind, &nonce, &pubnonce, &pubkey, &pubnonce)?;
		let kernel = Kernel::new(excess.clone(), sig.clone(), fee, 0);

		let mut tx = Transaction::new_with_offset(offset.clone());
		tx.add_input(input.clone())?;
		tx.add_output(output1.clone(), rp1.clone())?;
		tx.add_output(output2.clone(), rp2.clone())?;
		tx.add_output(output3.clone(), rp3.clone())?;
		tx.add_kernel(kernel)?;
		assert!(tx.verify(&mut ctx, 1000).is_ok());

		let mut tx = Transaction::new_with_offset(offset);
		let kernel = Kernel::new(excess, sig, fee + 1, 0);
		tx.add_input(input)?;
		tx.add_output(output1, rp1)?;
		tx.add_output(output2, rp2)?;
		tx.add_output(output3, rp3)?;
		tx.add_kernel(kernel)?;
		assert!(tx.verify(&mut ctx, 1000).is_err());

		Ok(())
	}

	#[test]
	fn test_coinbase() -> Result<(), Error> {
		/*
		let mut ctx = Ctx::new()?;
		let blank_blind = SecretKey::new(&mut ctx);
		let input = ctx.commit(1, &blank_blind)?;
		let blind = SecretKey::new(&mut ctx);
		let cb_output = ctx.commit(501, &blind)?;
		let cb_rp = ctx.range_proof(501, &blind)?;
		let fee = 0;
		let excess_blind = ctx.blind_sum(&[&blank_blind], &[&blind])?;
		let excess = ctx.commit(0, &excess_blind)?;
		let msg = ctx.hash_kernel(&excess, fee, 0)?;
		let nonce = SecretKey::new(&mut ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let pubkey = PublicKey::from(&ctx, &excess_blind)?;
		let sig = ctx.sign_single(&msg, &excess_blind, &nonce, &pubnonce, &pubkey, &pubnonce)?;
		let kernel = Kernel::new(excess.clone(), sig.clone(), fee, 0);
		let mut tx = Transaction::new();
		tx.add_output(cb_output.clone(), cb_rp.clone())?;
		tx.add_input(input.clone())?;
		tx.add_kernel(kernel)?;
		assert!(tx.verify(&mut ctx, 500).is_ok());
			*/

		Ok(())
	}

	#[test]
	fn test_coinbase_no_inputs() -> Result<(), Error> {
		/*
		let mut ctx = Ctx::new()?;
		let blind = SecretKey::new(&mut ctx);
		let cb_output = ctx.commit(500, &blind)?;
		let cb_rp = ctx.range_proof(500, &blind)?;
		let fee = 0;
		let excess_blind = ctx.blind_sum(&[], &[&blind])?;
		let excess = ctx.commit(0, &excess_blind)?;
		let msg = ctx.hash_kernel(&excess, fee, KERNEL_FEATURE_COINBASE)?;
		let nonce = SecretKey::new(&mut ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let pubkey = PublicKey::from(&ctx, &excess_blind)?;
		let sig = ctx.sign_single(&msg, &excess_blind, &nonce, &pubnonce, &pubkey, &pubnonce)?;
		let kernel = Kernel::new(excess.clone(), sig.clone(), fee, KERNEL_FEATURE_COINBASE);
		let mut tx = Transaction::new();
		tx.add_output(cb_output.clone(), cb_rp.clone())?;
		tx.add_kernel(kernel)?;
		assert!(tx.verify(&mut ctx, 500).is_ok());

		let mut ctx = Ctx::new()?;
		let blind = SecretKey::new(&mut ctx);
		let input = ctx.commit(0, &blind)?;
		let fee = 0;
		let excess_blind = ctx.blind_sum(&[&blind], &[])?;
		let excess = ctx.commit(0, &excess_blind)?;
		let msg = ctx.hash_kernel(&excess, fee, KERNEL_FEATURE_PLAIN)?;
		let nonce = SecretKey::new(&mut ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let pubkey = PublicKey::from(&ctx, &excess_blind)?;
		let sig = ctx.sign_single(&msg, &excess_blind, &nonce, &pubnonce, &pubkey, &pubnonce)?;
		let kernel = Kernel::new(excess.clone(), sig.clone(), fee, KERNEL_FEATURE_PLAIN);
		let mut tx = Transaction::new();
		tx.add_input(input.clone())?;
		tx.add_kernel(kernel)?;
		// all transactions must have an output
		assert_eq!(
			tx.verify(&mut ctx, 500).unwrap_err(),
			Error::new(InvalidTransaction)
		);

		// two outputs ok
		let mut ctx = Ctx::new()?;
		let blind = SecretKey::new(&mut ctx);
		let cb_output = ctx.commit(500, &blind)?;
		let cb_rp = ctx.range_proof(500, &blind)?;
		let blind2 = SecretKey::new(&mut ctx);
		let output2 = ctx.commit(500, &blind2)?;
		let rp2 = ctx.range_proof(500, &blind2)?;
		let fee = 0;
		let excess_blind = ctx.blind_sum(&[], &[&blind, &blind2])?;
		let excess = ctx.commit(0, &excess_blind)?;
		let msg = ctx.hash_kernel(&excess, fee, KERNEL_FEATURE_COINBASE)?;
		let nonce = SecretKey::new(&mut ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let pubkey = PublicKey::from(&ctx, &excess_blind)?;
		let sig = ctx.sign_single(&msg, &excess_blind, &nonce, &pubnonce, &pubkey, &pubnonce)?;
		let kernel = Kernel::new(excess.clone(), sig.clone(), fee, KERNEL_FEATURE_COINBASE);
		let mut tx = Transaction::new();
		tx.add_output(cb_output.clone(), cb_rp.clone())?;
		tx.add_output(output2.clone(), rp2.clone())?;
		tx.add_kernel(kernel)?;
		assert!(tx.verify(&mut ctx, 1000).is_ok());
			*/

		Ok(())
	}

	#[test]
	fn test_tx_merge() -> Result<(), Error> {
		// create our crypto context
		let mut ctx = Ctx::new()?;

		// create a slate with a fee of 10 coins
		let mut slate = Slate::new(10, SecretKey::new(&mut ctx));

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
		let mut tx = slate.finalize(&mut ctx)?;
		// confirm the transaction is valid
		assert!(tx.verify(&mut ctx, 1000).is_ok());

		let mut slate = Slate::new(20, SecretKey::new(&mut ctx));

		let user1_sec_nonce = SecretKey::new(&mut ctx); // random nonce
		let input = kc1.derive_key(&ctx, &[1, 0]);
		let change_output = kc1.derive_key(&ctx, &[1, 1]);

		let user1_id = slate.commit(
			&mut ctx,
			&[(&input, 200)],
			&[(&change_output, 29)],
			&user1_sec_nonce,
		)?;

		// now it's user2's turn
		let user2_sec_nonce = SecretKey::new(&mut ctx);
		let output = kc2.derive_key(&ctx, &[1, 0]);

		let user2_id = slate.commit(&mut ctx, &[], &[(&output, 151)], &user2_sec_nonce)?;
		slate.sign(&mut ctx, user2_id, &[], &[&output], &user2_sec_nonce)?;

		slate.sign(
			&mut ctx,
			user1_id,
			&[&input],
			&[&change_output],
			&user1_sec_nonce,
		)?;

		let tx2 = slate.finalize(&mut ctx)?;
		assert!(tx2.verify(&mut ctx, 1000).is_ok());

		assert_eq!(tx.outputs.len(), 2);
		assert_eq!(tx.inputs.len(), 1);
		assert_eq!(tx.kernels.len(), 1);

		assert_eq!(tx2.outputs.len(), 2);
		assert_eq!(tx2.inputs.len(), 1);
		assert_eq!(tx2.kernels.len(), 1);

		tx.merge(&mut ctx, tx2)?;
		assert!(tx.verify(&mut ctx, 100).is_ok());

		assert_eq!(tx.outputs.len(), 4);
		assert_eq!(tx.inputs.len(), 2);
		assert_eq!(tx.kernels.len(), 2);

		Ok(())
	}

	#[test]
	fn test_negate_kernels() -> Result<(), Error> {
		// create our crypto context
		let mut ctx = Ctx::new()?;
		let offset0 = SecretKey::new(&mut ctx);
		let mut offset1 = offset0.clone();
		offset1.negate(&mut ctx)?;

		// create a slate with a fee of 10 coins
		let mut slate = Slate::new(10, offset0);

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
		let mut tx0 = slate.finalize(&mut ctx)?;
		// confirm the transaction is valid
		assert!(tx0.verify(&mut ctx, 500).is_ok());

		// create a slate with a fee of 10 coins
		let mut slate = Slate::new(10, offset1);

		// user1 initiates request by offering to pay 100 coins with change of 10
		let kc1 = KeyChain::from_seed([2u8; 32])?;
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
		let kc2 = KeyChain::from_seed([3u8; 32])?;
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
		let mut tx1 = slate.finalize(&mut ctx)?;
		// confirm the transaction is valid
		assert!(tx1.verify(&mut ctx, 500).is_ok());

		assert_eq!(tx1.inputs.len(), 1);
		assert_eq!(tx1.outputs.len(), 2);
		assert_eq!(tx1.kernels.len(), 1);
		assert!(tx1.offset.is_some());
		assert_eq!(tx0.inputs.len(), 1);
		assert_eq!(tx0.outputs.len(), 2);
		assert_eq!(tx0.kernels.len(), 1);
		assert!(tx0.offset.is_some());

		tx1.merge(&mut ctx, tx0)?;
		tx1.offset = None;
		assert!(tx1.verify(&mut ctx, 500).is_ok());

		Ok(())
	}
		*/
}
