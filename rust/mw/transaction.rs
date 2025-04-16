use core::ptr::copy_nonoverlapping;
use crypto::{Commitment, Ctx, RangeProof, SecretKey};
use mw::kernel::Kernel;
use prelude::*;
use util::rbtree::{RbNodePair, RbTree, RbTreeNode};

pub struct Transaction {
	inputs: Vec<Commitment>,
	outputs: Vec<(Commitment, RangeProof)>,
	kernels_rb: RbTree<Kernel>,
	offset: Option<SecretKey>,
}

impl Drop for Transaction {
	fn drop(&mut self) {
		let root = self.kernels_rb.root();
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
			kernels_rb: self.kernels_rb.try_clone()?,
			offset: self.offset.clone(),
		})
	}
}

impl Transaction {
	pub fn new(skey: SecretKey) -> Self {
		Self {
			inputs: Vec::new(),
			outputs: Vec::new(),
			kernels_rb: RbTree::new(),
			offset: Some(skey),
		}
	}

	pub fn empty() -> Self {
		Self {
			inputs: Vec::new(),
			outputs: Vec::new(),
			kernels_rb: RbTree::new(),
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
		&self.kernels_rb
	}

	pub fn offset(&self) -> Option<&SecretKey> {
		self.offset.as_ref()
	}

	fn append_kernels(&mut self, ctx: &Ctx, node: Ptr<RbTreeNode<Kernel>>) -> Result<(), Error> {
		if !node.right.is_null() {
			self.append_kernels(ctx, node.right)?;
		}
		if !node.left.is_null() {
			self.append_kernels(ctx, node.left)?;
		}

		let ptr = Ptr::alloc(RbTreeNode::new(node.value.clone()))?;
		let mut search = move |base: Ptr<RbTreeNode<Kernel>>, value: Ptr<RbTreeNode<Kernel>>| {
			let mut is_right = false;
			let mut cur = base;
			let mut parent = Ptr::null();

			while !cur.is_null() {
				let cmp = (*value).value.cmp(&(*cur).value);
				if cmp == Ordering::Equal {
					break;
				} else if cmp == Ordering::Less {
					parent = cur;
					is_right = false;
					cur = cur.left;
				} else {
					parent = cur;
					is_right = true;
					cur = cur.right;
				}
			}

			RbNodePair {
				cur,
				parent,
				is_right,
			}
		};

		self.kernels_rb.insert(ptr, &mut search);
		Ok(())
	}

	pub fn merge(&mut self, ctx: &Ctx, t: Transaction) -> Result<(), Error> {
		self.append_kernels(ctx, t.kernels_rb.root())?;

		let olen = t.outputs.len();
		let ilen = t.inputs.len();
		match self.inputs.append(&t.inputs) {
			Ok(_) => {}
			Err(e) => {
				// TODO: rollback kernels
				return Err(e);
			}
		}
		match self.outputs.append(&t.outputs) {
			Ok(_) => {}
			Err(e) => {
				// TODO: rollback kernels
				self.inputs.truncate(self.inputs.len() - ilen)?;
				return Err(e);
			}
		}

		match &self.offset {
			Some(self_offset) => match &t.offset {
				Some(t_offset) => {
					let offset = match ctx.blind_sum(&[&self_offset, &t_offset], &[]) {
						Ok(o) => o,
						Err(e) => {
							// TODO: rollback kernels
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
		let mut search = move |base: Ptr<RbTreeNode<Kernel>>, value: Ptr<RbTreeNode<Kernel>>| {
			let mut is_right = false;
			let mut cur = base;
			let mut parent = Ptr::null();

			while !cur.is_null() {
				let cmp = (*value).value.cmp(&(*cur).value);
				if cmp == Ordering::Equal {
					break;
				} else if cmp == Ordering::Less {
					parent = cur;
					is_right = false;
					cur = cur.left;
				} else {
					parent = cur;
					is_right = true;
					cur = cur.right;
				}
			}

			RbNodePair {
				cur,
				parent,
				is_right,
			}
		};

		let ptr = Ptr::alloc(RbTreeNode::new(kernel.clone()))?;
		match self.kernels_rb.insert(ptr, &mut search) {
			Some(kernel) => {
				// if we append a kernel that's already present
				// release the previously inserted version
				// this should not be happening, but try to manage
				// memory correctly here
				kernel.release();
			}
			None => {}
		}

		Ok(())
	}

	pub fn kernel_merkle_root(&self, ctx: &mut Ctx) -> Result<[u8; 32], Error> {
		let kernels = self.kernels();
		if kernels.len() == 0 {
			return Ok([0u8; 32]);
		}
		// Initialize Vec for kernel hashes
		let mut leaves = Vec::with_capacity(kernels.len())?;

		// Copy kernel hashes from red-black tree to Vec
		for k in kernels.iter() {
			ctx.sha3().reset();
			let mut hash = [0u8; 32];
			k.sha3(ctx.sha3());
			ctx.sha3().finalize(&mut hash)?;
			leaves.push(hash)?;
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
				unsafe {
					copy_nonoverlapping(left.as_ptr(), input.as_mut_ptr(), 32);
					copy_nonoverlapping(right.as_ptr(), input.as_mut_ptr().add(32), 32);
				}

				// Hash the pair
				ctx.sha3().reset();
				let mut hash = [0u8; 32];
				ctx.sha3().update(&input);
				ctx.sha3().finalize(&mut hash)?;

				next_level.push(hash)?;
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

	pub fn verify(&self, ctx: &mut Ctx, overage: u64) -> Result<(), Error> {
		if self.kernels_rb.root().is_null() {
			return Err(Error::new(KernelNotFound));
		}
		if self.outputs.len() == 0 {
			return Err(Error::new(InvalidTransaction));
		}
		for i in 0..self.outputs.len() {
			let output = &self.outputs[i];
			ctx.verify_range_proof(&output.0, &output.1)?;
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
		let mut fee = 0;
		self.verify_kernels(self.kernels_rb.root(), ctx, &mut fee, &mut output_commits)?;

		let inp = if input_commits.len() == 0 {
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
			inp,
			output_commits.slice(0, output_commits.len()),
			adjustment,
		)?;

		Ok(())
	}

	pub fn fees(&self) -> u64 {
		let mut fee = 0;
		let root = self.kernels_rb.root();
		if !root.is_null() {
			self.fee_node(root, &mut fee);
		}
		fee
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

	fn verify_kernels(
		&self,
		ptr: Ptr<RbTreeNode<Kernel>>,
		ctx: &mut Ctx,
		fee: &mut u64,
		output_commits: &mut Vec<Commitment>,
	) -> Result<(), Error> {
		if !ptr.right.is_null() {
			self.verify_kernels(ptr.right, ctx, fee, output_commits)?;
		}
		if !ptr.left.is_null() {
			self.verify_kernels(ptr.left, ctx, fee, output_commits)?;
		}
		output_commits.push(ptr.value.excess().clone())?;
		let msg = ctx.hash_kernel(ptr.value.excess(), ptr.value.fee(), ptr.value.features())?;
		*fee += ptr.value.fee();
		ptr.value.verify(ctx, &msg)?;
		Ok(())
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
	use crypto::{Ctx, PublicKey, SecretKey};
	use std::ffi::getalloccount;

	#[test]
	fn test_transaction() -> Result<(), Error> {
		let init = unsafe { getalloccount() };
		{
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
		}
		assert_eq!(init, unsafe { getalloccount() });

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
			let aggsig =
				ctx_user1.aggregate_signatures(&partial_sigs.slice(0, 2), &pubnonce_sum)?;

			// Build transaction
			let kernel = Kernel::new(excess_sum, aggsig, fee, 0);
			let mut tx = Transaction::new(offset);
			tx.add_kernel(kernel)?;
			tx.add_output(change_output, change_range_proof)?;
			tx.add_output(output, range_proof)?;
			tx.add_input(input)?;

			assert!(tx.verify(&mut ctx_user2, 0).is_ok());
		}
		assert_eq!(init, unsafe { getalloccount() });

		Ok(())
	}

	#[test]
	fn test_empty() -> Result<(), Error> {
		let mut ctx = Ctx::new()?;
		let mut tx = Transaction::empty();

		let blind_output = SecretKey::gen(&ctx);
		let output = ctx.commit(2000, &blind_output)?;
		let range_proof = ctx.range_proof(2000, &blind_output)?;
		let nonce = SecretKey::gen(&ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let excess_blind = ctx.blind_sum(&[], &[&blind_output])?;
		let pub_blind = PublicKey::from(&ctx, &excess_blind)?;
		let excess = ctx.commit(0, &excess_blind)?;
		let msg = ctx.hash_kernel(&excess, 0, 0)?;
		let sig = ctx.sign_single(&msg, &excess_blind, &nonce, &pubnonce, &pub_blind)?;
		let kernel = Kernel::new(excess, sig, 0, 0);
		tx.add_kernel(kernel)?;
		tx.add_output(output, range_proof)?;

		assert!(tx.verify(&mut ctx, 2000).is_ok());
		assert!(tx.verify(&mut ctx, 2001).is_err());
		Ok(())
	}

	#[test]
	fn test_kernel_merkle_roots() -> Result<(), Error> {
		let mut ctx = Ctx::new()?;
		let mut tx = Transaction::empty();

		let blind_output = SecretKey::gen(&ctx);
		let output = ctx.commit(2000, &blind_output)?;
		let range_proof = ctx.range_proof(2000, &blind_output)?;
		let nonce = SecretKey::gen(&ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let excess_blind = ctx.blind_sum(&[], &[&blind_output])?;
		let pub_blind = PublicKey::from(&ctx, &excess_blind)?;
		let excess = ctx.commit(0, &excess_blind)?;
		let msg = ctx.hash_kernel(&excess, 0, 0)?;
		let sig = ctx.sign_single(&msg, &excess_blind, &nonce, &pubnonce, &pub_blind)?;
		let kernel = Kernel::new(excess, sig, 0, 0);
		tx.add_kernel(kernel.clone())?;
		tx.add_output(output, range_proof)?;

		assert!(tx.verify(&mut ctx, 2000).is_ok());

		ctx.sha3().reset();
		let mut hash = [0u8; 32];
		kernel.sha3(ctx.sha3());
		ctx.sha3().finalize(&mut hash)?;
		assert_eq!(hash, tx.kernel_merkle_root(&mut ctx)?);

		Ok(())
	}
}
