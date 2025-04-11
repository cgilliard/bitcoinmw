use crypto::ctx::Ctx;
use crypto::kernel::Kernel;
use crypto::pedersen::Commitment;
use crypto::range_proof::RangeProof;
use mw::constants::KERNEL_FEATURE_COINBASE;
use prelude::*;

pub struct Transaction {
	inputs: Vec<Commitment>,
	outputs: Vec<(Commitment, RangeProof)>,
	kernels: Vec<Kernel>,
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
		})
	}
}

impl Transaction {
	pub fn new() -> Self {
		Self {
			inputs: Vec::new(),
			outputs: Vec::new(),
			kernels: Vec::new(),
		}
	}

	pub fn merge(&mut self, t: Transaction) -> Result<(), Error> {
		let mut kernels = self.kernels.try_clone()?;
		let mut inputs = self.inputs.try_clone()?;
		let mut outputs = self.outputs.try_clone()?;

		kernels.append(&t.kernels)?;
		inputs.append(&t.inputs)?;
		outputs.append(&t.outputs)?;

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

	pub fn verify(&self, ctx: &mut Ctx, block_reward: u64) -> Result<(), Error> {
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
		let mut fee = 0;
		let mut block_reward_overage: i128 = 0;
		for i in 0..self.kernels.len() {
			output_commits.push(self.kernels[i].excess())?;
			let msg = ctx.hash_kernel(
				self.kernels[i].excess(),
				self.kernels[i].fee(),
				self.kernels[i].features(),
			)?;
			if self.kernels[i].features() == KERNEL_FEATURE_COINBASE {
				if block_reward_overage != 0 {
					return Err(Error::new(MultipleCoinbase));
				}
				block_reward_overage -= block_reward as i128;
			}
			self.kernels[i].verify(ctx, &msg)?;
			fee += self.kernels[i].fee();
		}

		let inp = if input_commits.len() == 0 {
			&[]
		} else {
			input_commits.slice(0, input_commits.len())
		};

		if !ctx.verify_balance(
			inp,
			output_commits.slice(0, output_commits.len()),
			fee as i128 + block_reward_overage,
		)? {
			return Err(Error::new(InvalidTransaction));
		}

		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crypto::ctx::Ctx;
	use crypto::keys::{PublicKey, SecretKey};
	use mw::constants::KERNEL_FEATURE_PLAIN;
	use mw::keychain::KeyChain;
	use mw::slate::Slate;

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
		let msg = ctx.hash_kernel(&excess, fee, KERNEL_FEATURE_PLAIN)?;

		let nonce = SecretKey::new(&mut ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let pubkey = PublicKey::from(&ctx, &excess_blind)?;

		let sig = ctx.sign_single(&msg, &excess_blind, &nonce, &pubnonce, &pubkey, &pubnonce)?;
		let kernel = Kernel::new(excess.clone(), sig.clone(), fee, KERNEL_FEATURE_PLAIN);

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
	fn test_coinbase() -> Result<(), Error> {
		let mut ctx = Ctx::new()?;
		let blank_blind = SecretKey::new(&mut ctx);
		let input = ctx.commit(1, &blank_blind)?;
		let blind = SecretKey::new(&mut ctx);
		let cb_output = ctx.commit(501, &blind)?;
		let cb_rp = ctx.range_proof(501, &blind)?;
		let fee = 0;
		let excess_blind = ctx.blind_sum(&[&blank_blind], &[&blind])?;
		let excess = ctx.commit(0, &excess_blind)?;
		let msg = ctx.hash_kernel(&excess, fee, KERNEL_FEATURE_COINBASE)?;
		let nonce = SecretKey::new(&mut ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let pubkey = PublicKey::from(&ctx, &excess_blind)?;
		let sig = ctx.sign_single(&msg, &excess_blind, &nonce, &pubnonce, &pubkey, &pubnonce)?;
		let kernel = Kernel::new(excess.clone(), sig.clone(), fee, KERNEL_FEATURE_COINBASE);
		let mut tx = Transaction::new();
		tx.add_output(cb_output.clone(), cb_rp.clone())?;
		tx.add_input(input.clone())?;
		tx.add_kernel(kernel)?;
		assert!(tx.verify(&mut ctx, 500).is_ok());

		Ok(())
	}

	#[test]
	fn test_coinbase_no_inputs() -> Result<(), Error> {
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

		Ok(())
	}

	#[test]
	fn test_tx_merge() -> Result<(), Error> {
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
		let mut tx = slate.finalize(&mut ctx)?;
		// confirm the transaction is valid
		assert!(tx.verify(&mut ctx, 1000).is_ok());

		let mut slate = Slate::new(20);

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

		tx.merge(tx2)?;
		assert!(tx.verify(&mut ctx, 100).is_ok());

		assert_eq!(tx.outputs.len(), 4);
		assert_eq!(tx.inputs.len(), 2);
		assert_eq!(tx.kernels.len(), 2);

		Ok(())
	}
}
