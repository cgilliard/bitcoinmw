use crypto::{Commitment, Ctx, Message, RangeProof, SecretKey, Sha3_256};
use mw::Kernel;
use prelude::*;
use std::misc::{slice_copy, subslice_mut};
use util::{RbTree, RbTreeNode};

pub struct Transaction {
	outputs: Vec<(Commitment, RangeProof)>,
	inputs: Vec<Commitment>,
	kernels: RbTree<Kernel>,
	offset: Option<SecretKey>,
}

impl Drop for Transaction {
	fn drop(&mut self) {
		let root = self.kernels.root();
		if !root.is_null() {
			self.clear_kernels(root);
		}
	}
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

	pub fn kernel_merkle_root(&self) -> Result<Message, Error> {
		let kernels = self.kernels();
		if kernels.len() == 0 {
			return Ok(Message::zero());
		}
		// Initialize Vec for kernel hashes
		let mut leaves = Vec::with_capacity(kernels.len())?;

		// Copy kernel hashes from red-black tree to Vec
		for k in kernels.iter() {
			leaves.push(k.message())?;
		}

		// Build Merkle tree bottom-up
		while leaves.len() > 1 {
			let mut next_level = Vec::with_capacity((leaves.len() + 1) / 2)?;

			for i in 0..leaves.len() {
				if i % 2 != 0 {
					continue;
				}
				let left = leaves[i];
				let right = if i + 1 < leaves.len() {
					leaves[i + 1]
				} else {
					left // Duplicate last hash for odd number of leaves
				};

				// Concatenate left || right
				let mut input = [0u8; 64];
				slice_copy(left.as_ref(), &mut input, 32)?;
				let mut input_end = subslice_mut(&mut input, 32, 32)?;
				slice_copy(right.as_ref(), &mut input_end, 32)?;

				// Hash the pair
				let sha3 = Sha3_256::new();
				sha3.update(&input);
				let hash = sha3.finalize();
				next_level.push(Message::new(hash))?;
			}
			// Replace leaves with next level
			leaves = next_level;
		}

		// there should be exactly 1 leaf here, but avoid panic by returning IllegalState
		match leaves.len() {
			1 => Ok(leaves[0]),
			_ => Err(Error::new(IllegalState)),
		}
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

	fn clear_kernels(&mut self, ptr: Ptr<RbTreeNode<Kernel>>) {
		if !ptr.right.is_null() {
			self.clear_kernels(ptr.right);
		}
		if !ptr.left.is_null() {
			self.clear_kernels(ptr.left);
		}

		ptr.release();
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crypto::{PublicKey, Signature};

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

	#[test]
	fn test_interactive() -> Result<(), Error> {
		let init = unsafe { getalloccount() };
		{
			let ctx_user1 = Ctx::new()?;
			let mut ctx_user2 = Ctx::new()?;

			// User 1: input and change
			let offset = SecretKey::gen(&ctx_user1);
			let blind_input = SecretKey::gen(&ctx_user1);
			let blind_change = SecretKey::gen(&ctx_user1);
			let input = ctx_user1.commit(5_000, &blind_input)?;
			let change_output = ctx_user1.commit(1_000, &blind_change)?;
			let change_range_proof = ctx_user1.range_proof(1_000, &blind_change)?;
			let nonce_user1 = SecretKey::gen(&ctx_user1);
			let pub_nonce_user1 = PublicKey::from(&ctx_user1, &nonce_user1)?;
			let excess_blind_user1 =
				ctx_user1.blind_sum(&[&blind_input], &[&blind_change, &offset])?;
			let pub_blind_user1 = PublicKey::from(&ctx_user1, &excess_blind_user1)?;
			let excess_user1 = ctx_user1.commit(0, &excess_blind_user1)?;
			let fee = 50;

			// User 2: output
			let blind_output = SecretKey::gen(&ctx_user2);
			let output = ctx_user2.commit(3_950, &blind_output)?;
			let range_proof = ctx_user2.range_proof(3_950, &blind_output)?;
			let nonce_user2 = SecretKey::gen(&ctx_user2);
			let pub_nonce_user2 = PublicKey::from(&ctx_user2, &nonce_user2)?;
			let excess_blind_user2 = ctx_user2.blind_sum(&[], &[&blind_output])?;
			let pub_blind_user2 = PublicKey::from(&ctx_user2, &excess_blind_user2)?;
			let excess_user2 = ctx_user2.commit(0, &excess_blind_user2)?;

			// Aggregate
			let pubnonce_sum = pub_nonce_user1.combine(&ctx_user2, &pub_nonce_user2)?;
			let pubblind_sum = pub_blind_user1.combine(&ctx_user2, &pub_blind_user2)?;
			let excess_sum = excess_user2.combine(&ctx_user2, &excess_user1)?;
			let msg = Kernel::message_for(&excess_sum, fee, 0);

			let sig2 = ctx_user2.sign(
				&msg,
				&excess_blind_user2,
				&nonce_user2,
				&pubnonce_sum,
				&pubblind_sum,
			)?;
			let sig1 = ctx_user1.sign(
				&msg,
				&excess_blind_user1,
				&nonce_user1,
				&pubnonce_sum,
				&pubblind_sum,
			)?;

			let partial_sigs = vec![&sig1, &sig2]?;
			let aggsig =
				ctx_user1.aggregate_signatures(&partial_sigs.slice(0, 2), &pubnonce_sum)?;

			// Build transaction
			let kernel = Kernel::new(excess_sum, aggsig, fee, 0);
			let mut tx = Transaction::new(offset);
			tx.add_kernel(kernel)?;
			tx.add_output(change_output, change_range_proof)?;
			tx.add_output(output, range_proof)?;
			tx.add_input(input)?;

			assert!(tx.validate(&mut ctx_user2, 0).is_ok());
		}
		assert_eq!(init, unsafe { getalloccount() });
		Ok(())
	}

	#[test]
	fn test_empty() -> Result<(), Error> {
		let ctx = Ctx::new()?;
		let mut tx = Transaction::empty();

		let fee = 0;
		let features = 1;
		let blind_output = SecretKey::gen(&ctx);
		let output = ctx.commit(2000, &blind_output)?;
		let range_proof = ctx.range_proof(2000, &blind_output)?;
		let nonce = SecretKey::gen(&ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let excess_blind = ctx.blind_sum(&[], &[&blind_output])?;
		let pub_blind = PublicKey::from(&ctx, &excess_blind)?;
		let excess = ctx.commit(0, &excess_blind)?;
		let msg = Kernel::message_for(&excess, fee, features);
		let sig = ctx.sign(&msg, &excess_blind, &nonce, &pubnonce, &pub_blind)?;
		let kernel = Kernel::new(excess, sig, fee, features);
		tx.add_kernel(kernel)?;
		tx.add_output(output, range_proof)?;

		assert!(tx.validate(&ctx, 2000).is_ok());
		assert!(tx.validate(&ctx, 2001).is_err());
		Ok(())
	}

	#[test]
	fn test_kernel_merkle_roots() -> Result<(), Error> {
		let ctx = Ctx::new()?;
		let mut tx = Transaction::empty();

		let blind_output = SecretKey::gen(&ctx);
		let output = ctx.commit(2000, &blind_output)?;
		let range_proof = ctx.range_proof(2000, &blind_output)?;
		let nonce = SecretKey::gen(&ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let excess_blind = ctx.blind_sum(&[], &[&blind_output])?;
		let pub_blind = PublicKey::from(&ctx, &excess_blind)?;
		let excess = ctx.commit(0, &excess_blind)?;
		let msg = Kernel::message_for(&excess, 0, 0);
		let sig = ctx.sign(&msg, &excess_blind, &nonce, &pubnonce, &pub_blind)?;
		let kernel = Kernel::new(excess, sig, 0, 0);
		tx.add_kernel(kernel.clone())?;
		tx.add_output(output, range_proof)?;

		assert!(tx.validate(&ctx, 2000).is_ok());

		let hash = kernel.message();
		assert_eq!(hash, tx.kernel_merkle_root()?);

		let sig_zero = Signature::new();
		let mut tx = Transaction::empty();
		let blind1 = SecretKey::gen(&ctx);
		let commit1 = ctx.commit(0, &blind1)?;
		let kernel1 = Kernel::new(commit1, sig_zero.clone(), 0, 0);
		tx.add_kernel(kernel1.clone())?;
		let blind2 = SecretKey::gen(&ctx);
		let commit2 = ctx.commit(0, &blind2)?;
		let kernel2 = Kernel::new(commit2, sig_zero.clone(), 0, 0);
		tx.add_kernel(kernel2.clone())?;

		let sha3 = Sha3_256::new();
		// order matters
		if kernel1 < kernel2 {
			sha3.update(kernel1.message().as_ref());
			sha3.update(kernel2.message().as_ref());
		} else {
			sha3.update(kernel2.message().as_ref());
			sha3.update(kernel1.message().as_ref());
		}

		assert_eq!(sha3.finalize(), tx.kernel_merkle_root()?.as_ref());

		Ok(())
	}
}
