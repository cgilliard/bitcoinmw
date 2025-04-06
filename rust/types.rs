#![allow(dead_code)]

#[repr(C)]
pub struct Secp256k1Context(usize);
#[repr(C)]
pub struct Secp256k1AggsigContext(usize);
#[repr(C)]
pub struct CsprngCtx(usize);
#[repr(C)]
pub struct PublicKeyUncompressed(pub [u8; 64]);
#[repr(C)]
pub struct SecretKey(pub [u8; 32]);
#[repr(C)]
pub struct AggSigPartialSignature(pub [u8; 32]);
#[repr(C)]
pub struct Signature(pub [u8; 64]);
#[repr(C)]
pub struct CommitmentUncompressed(pub [u8; 64]);
#[repr(C)]
pub struct ScratchSpace(usize);
#[repr(C)]
pub struct BulletproofGenerators(usize);
#[repr(C)]
pub struct Commitment(pub [u8; 33]);
#[repr(C)]
pub struct PublicKey(pub [u8; 33]);
