use mw::transaction::Transaction;
use prelude::*;

struct ParticipantData {}

pub struct Slate {
	pdata: Vec<ParticipantData>,
	tx: Option<Transaction>,
}
