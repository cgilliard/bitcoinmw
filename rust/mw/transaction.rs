use crypto::{Commitment, Ctx, RangeProof, SecretKey};
use mw::Kernel;
use prelude::*;
use util::{RbTree, RbTreeNode};

pub struct Transaction {
	outputs: Vec<(Commitment, RangeProof)>,
	inputs: Vec<Commitment>,
	kernels: RbTree<Kernel>,
	offset: Option<SecretKey>,
}

impl Drop for Transaction {
	fn drop(&mut self) {}
}

impl Transaction {
	pub fn new(offset: SecretKey) -> Self {
		Self {
			outputs: Vec::new(),
			inputs: Vec::new(),
			kernels: RbTree::new(),
			offset: Some(offset),
		}
	}

	pub fn empty() -> Self {
		Self {
			outputs: Vec::new(),
			inputs: Vec::new(),
			kernels: RbTree::new(),
			offset: None,
		}
	}

	pub fn outputs(&self) -> &Vec<(Commitment, RangeProof)> {
		&self.outputs
	}

	pub fn inputs(&self) -> &Vec<Commitment> {
		&self.inputs
	}

	pub fn kernels(&self) -> &RbTree<Kernel> {
		&self.kernels
	}

	pub fn offset(&self) -> Option<&SecretKey> {
		self.offset.as_ref()
	}

	pub fn merge(&mut self, ctx: &Ctx, t: Transaction) -> Result<(), Error> {
		let olen = t.outputs.len();
		let ilen = t.inputs.len();
		match self.inputs.extend(&t.inputs) {
			Ok(_) => {}
			Err(e) => {
				return Err(e);
			}
		}
		match self.outputs.extend(&t.outputs) {
			Ok(_) => {}
			Err(e) => {
				self.inputs.truncate(self.inputs.len() - ilen)?;
				return Err(e);
			}
		}

		let original_offset = self.offset.clone();
		match &self.offset {
			Some(self_offset) => match &t.offset {
				Some(t_offset) => {
					let offset = match ctx.blind_sum(&[&self_offset, &t_offset], &[]) {
						Ok(o) => o,
						Err(e) => {
							self.inputs.truncate(self.inputs.len() - ilen)?;
							self.outputs.truncate(self.outputs.len() - olen)?;
							return Err(e);
						}
					};
					self.offset = Some(offset);
				}
				None => {}
			},
			None => match &t.offset {
				Some(t_offset) => self.offset = Some(t_offset.clone()),
				None => {}
			},
		}

		match self.append_kernels(ctx, t.kernels.root()) {
			Ok(_) => {}
			Err(e) => {
				self.offset = original_offset;
				self.inputs.truncate(self.inputs.len() - ilen)?;
				self.outputs.truncate(self.outputs.len() - olen)?;
				return Err(e);
			}
		}

		Ok(())
	}

	pub fn set_offset_zero(&mut self) {
		self.offset = None;
	}

	pub fn add_input(&mut self, input: Commitment) -> Result<(), Error> {
		self.inputs.push(input)
	}

	pub fn add_output(&mut self, output: Commitment, proof: RangeProof) -> Result<(), Error> {
		self.outputs.push((output, proof))
	}

	pub fn add_kernel(&mut self, kernel: Kernel) -> Result<(), Error> {
		let ptr = Ptr::alloc(RbTreeNode::new(kernel.clone()))?;
		match self.kernels.insert(ptr) {
			Some(kernel) => {
				kernel.release();
				Err(Error::new(Duplicate))
			}
			None => Ok(()),
		}
	}

	pub fn kernel_merkle_root(&self, _ctx: &mut Ctx) -> Result<[u8; 32], Error> {
		Err(Error::new(Todo))
	}

	pub fn validate(&self, ctx: &Ctx, overage: u64) -> Result<(), Error> {
		if self.kernels.root().is_null() || self.outputs.len() == 0 {
			return Err(Error::new(NotFound));
		}
		for i in 0..self.outputs.len() {
			let pair = &self.outputs[i];
			ctx.verify_range_proof(&pair.0, &pair.1)?;
		}
		let mut input_commits: Vec<Commitment> = Vec::new();
		for i in 0..self.inputs.len() {
			input_commits.push(self.inputs[i].clone())?;
		}
		let mut output_commits: Vec<Commitment> = Vec::new();
		for i in 0..self.outputs.len() {
			output_commits.push(self.outputs[i].0.clone())?;
		}

		if let Some(offset) = &self.offset {
			let offset_commit = ctx.commit(0, offset)?;
			output_commits.push(offset_commit)?;
		}

		let fee = self.fees();

		self.verify_kernels(self.kernels.root(), ctx, &mut output_commits)?;

		let inputs = if input_commits.len() == 0 {
			&[]
		} else {
			input_commits.slice(0, input_commits.len())
		};

		// if there's no overage (not coinbase), we just add fee
		// otherwise, we negate the overage
		let adjustment: i128 = if overage == 0 {
			fee as i128
		} else {
			-1i128 * (overage as i128)
		};

		ctx.verify_balance_owned(
			inputs,
			output_commits.slice(0, output_commits.len()),
			adjustment,
		)
	}

	pub fn fees(&self) -> u64 {
		let mut fee = 0;
		let root = self.kernels.root();
		if !root.is_null() {
			self.fee_node(root, &mut fee);
		}
		fee
	}

	fn verify_kernels(
		&self,
		ptr: Ptr<RbTreeNode<Kernel>>,
		ctx: &Ctx,
		output_commits: &mut Vec<Commitment>,
	) -> Result<(), Error> {
		if !ptr.right.is_null() {
			self.verify_kernels(ptr.right, ctx, output_commits)?;
		}
		if !ptr.left.is_null() {
			self.verify_kernels(ptr.left, ctx, output_commits)?;
		}
		output_commits.push(ptr.value.excess().clone())?;

		ptr.value.validate(ctx)
	}

	fn fee_node(&self, ptr: Ptr<RbTreeNode<Kernel>>, fee: &mut u64) {
		*fee += ptr.value.fee();
		if !ptr.right.is_null() {
			self.fee_node(ptr.right, fee);
		}
		if !ptr.left.is_null() {
			self.fee_node(ptr.left, fee);
		}
	}

	fn append_kernels(&mut self, ctx: &Ctx, node: Ptr<RbTreeNode<Kernel>>) -> Result<(), Error> {
		if !node.right.is_null() {
			self.append_kernels(ctx, node.right)?;
		}
		if !node.left.is_null() {
			self.append_kernels(ctx, node.left)?;
		}

		let ptr = Ptr::alloc(RbTreeNode::new(node.value.clone()))?;
		match self.kernels.insert(ptr) {
			Some(duplicate) => {
				duplicate.release();
				Err(Error::new(Duplicate))
			}
			None => Ok(()),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crypto::PublicKey;

	#[test]
	fn test_transaction1() -> Result<(), Error> {
		let ctx = Ctx::seed([0u8; 32], [0u8; 16])?;

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
		let features = 0;

		let excess_blind = ctx.blind_sum(&[&blind1], &[&blind2, &blind3, &blind4, &offset])?;
		let excess = ctx.commit(0, &excess_blind)?;
		let msg = Kernel::message_for(&excess, fee, features);

		let nonce = SecretKey::gen(&ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let pubkey = PublicKey::from(&ctx, &excess_blind)?;

		let sig = ctx.sign(&msg, &excess_blind, &nonce, &pubnonce, &pubkey)?;
		let kernel = Kernel::new(excess.clone(), sig.clone(), fee, 0);

		let mut tx = Transaction::new(offset.clone());
		tx.add_input(input.clone())?;
		tx.add_output(output1.clone(), rp1.clone())?;
		tx.add_output(output2.clone(), rp2.clone())?;
		tx.add_output(output3.clone(), rp3.clone())?;
		tx.add_kernel(kernel)?;
		assert!(tx.validate(&ctx, 0).is_ok());

		Ok(())
	}
}
