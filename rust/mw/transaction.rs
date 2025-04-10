use crypto::ctx::Ctx;
use crypto::kernel::Kernel;
use crypto::pedersen::Commitment;
use crypto::range_proof::RangeProof;
use prelude::*;

pub struct Transaction {
	inputs: Vec<Commitment>,
	outputs: Vec<(Commitment, RangeProof)>,
	kernel: Option<Kernel>,
}

impl Transaction {
	pub fn new() -> Self {
		Self {
			inputs: Vec::new(),
			outputs: Vec::new(),
			kernel: None,
		}
	}

	pub fn add_input(&mut self, input: Commitment) -> Result<(), Error> {
		self.inputs.push(input)
	}

	pub fn add_output(&mut self, output: Commitment, proof: RangeProof) -> Result<(), Error> {
		self.outputs.push((output, proof))
	}

	pub fn add_kernel(&mut self, kernel: Kernel) {
		self.kernel = Some(kernel);
	}

	pub fn verify(&self, ctx: &mut Ctx) -> Result<(), Error> {
		let kernel = match &self.kernel {
			Some(k) => k,
			None => return Err(Error::new(KernelNotFound)),
		};

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
		output_commits.push(kernel.excess())?;

		if !ctx.verify_balance(
			input_commits.slice(0, input_commits.len()),
			output_commits.slice(0, output_commits.len()),
			kernel.fee() as i128,
		)? {
			return Err(Error::new(InvalidTransaction));
		}

		// Verify signature
		let msg = ctx.hash_kernel(kernel.excess(), kernel.fee(), 0)?;
		kernel.verify(ctx, &msg)?;

		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crypto::ctx::Ctx;
	use crypto::keys::{PublicKey, SecretKey};

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
		let kernel = Kernel::new(excess.clone(), sig.clone(), fee);

		let mut tx = Transaction::new();
		tx.add_input(input.clone())?;
		tx.add_output(output1.clone(), rp1.clone())?;
		tx.add_output(output2.clone(), rp2.clone())?;
		tx.add_output(output3.clone(), rp3.clone())?;
		tx.add_kernel(kernel);
		assert!(tx.verify(&mut ctx).is_ok());

		let mut tx = Transaction::new();
		let kernel = Kernel::new(excess, sig, fee + 1);
		tx.add_input(input)?;
		tx.add_output(output1, rp1)?;
		tx.add_output(output2, rp2)?;
		tx.add_output(output3, rp3)?;
		tx.add_kernel(kernel);
		assert!(tx.verify(&mut ctx).is_err());

		Ok(())
	}
}
