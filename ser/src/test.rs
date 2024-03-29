// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[cfg(test)]
mod test {
	use crate::{deserialize, serialize, Reader, Serializable, Writer};
	use bmw_deps::rand;
	use bmw_err::*;
	use std::fmt::Debug;

	// type that can be used to generate an error
	#[derive(Debug, PartialEq)]
	struct SerErr {
		exp: u8,
		empty: u8,
	}

	impl Serializable for SerErr {
		fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
			// read data but return an error unless a specific value is set
			reader.expect_u8(99)?;
			reader.read_empty_bytes(1)?;
			Ok(Self { exp: 99, empty: 0 })
		}
		fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
			// write is regular with no errors
			writer.write_u8(self.exp)?;
			writer.write_u8(self.empty)?;
			Ok(())
		}
	}

	// helper function that serializes and deserializes a Serializable and tests them for
	// equality
	fn ser_helper<S: Serializable + Debug + PartialEq>(ser_out: S) -> Result<(), Error> {
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: S = deserialize(&mut &v[..])?;
		assert_eq!(ser_in, ser_out);
		Ok(())
	}

	// struct with all types
	#[derive(Debug, PartialEq)]
	struct SerAll {
		a: u8,
		b: i8,
		c: u16,
		d: i16,
		e: u32,
		f: i32,
		g: u64,
		h: i64,
		i: u128,
		j: i128,
		k: usize,
		l: bool,
		m: f64,
		n: char,
		v: Vec<u8>,
		o: Option<u8>,
	}

	// read/write with some added data to exercise all functions in the interface
	impl Serializable for SerAll {
		fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
			let a = reader.read_u8()?;
			let b = reader.read_i8()?;
			let c = reader.read_u16()?;
			let d = reader.read_i16()?;
			let e = reader.read_u32()?;
			let f = reader.read_i32()?;
			let g = reader.read_u64()?;
			let h = reader.read_i64()?;
			let i = reader.read_u128()?;
			let j = reader.read_i128()?;
			let k = reader.read_usize()?;
			let l = bool::read(reader)?;
			let m = f64::read(reader)?;
			let n = char::read(reader)?;
			let v = Vec::read(reader)?;
			let o = Option::read(reader)?;
			reader.expect_u8(100)?;
			assert_eq!(reader.read_u64()?, 4);
			reader.read_u8()?;
			reader.read_u8()?;
			reader.read_u8()?;
			reader.read_u8()?;
			reader.read_empty_bytes(10)?;

			let ret = Self {
				a,
				b,
				c,
				d,
				e,
				f,
				g,
				h,
				i,
				j,
				k,
				l,
				m,
				n,
				v,
				o,
			};

			Ok(ret)
		}
		fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
			writer.write_u8(self.a)?;
			writer.write_i8(self.b)?;
			writer.write_u16(self.c)?;
			writer.write_i16(self.d)?;
			writer.write_u32(self.e)?;
			writer.write_i32(self.f)?;
			writer.write_u64(self.g)?;
			writer.write_i64(self.h)?;
			writer.write_u128(self.i)?;
			writer.write_i128(self.j)?;
			writer.write_usize(self.k)?;
			bool::write(&self.l, writer)?;
			f64::write(&self.m, writer)?;
			char::write(&self.n, writer)?;
			Vec::write(&self.v, writer)?;
			Option::write(&self.o, writer)?;
			writer.write_u8(100)?;
			writer.write_bytes([1, 2, 3, 4])?;
			writer.write_empty_bytes(10)?;
			Ok(())
		}
	}
	#[test]
	fn test_ser() -> Result<(), Error> {
		// create a SerAll with random values
		let rand_u8: u8 = rand::random();
		let rand_ch: char = rand_u8 as char;
		let ser_out = SerAll {
			a: rand::random(),
			b: rand::random(),
			c: rand::random(),
			d: rand::random(),
			e: rand::random(),
			f: rand::random(),
			g: rand::random(),
			h: rand::random(),
			i: rand::random(),
			j: rand::random(),
			k: rand::random(),
			l: false,
			m: rand::random(),
			n: rand_ch,
			v: vec![rand::random(), rand::random(), rand::random()],
			o: Some(rand::random()),
		};

		// test it
		ser_helper(ser_out)?;

		// create again with some other options
		let rand_u8: u8 = rand::random();
		let rand_ch: char = rand_u8 as char;
		let ser_out = SerAll {
			a: rand::random(),
			b: rand::random(),
			c: rand::random(),
			d: rand::random(),
			e: rand::random(),
			f: rand::random(),
			g: rand::random(),
			h: rand::random(),
			i: rand::random(),
			j: rand::random(),
			k: rand::random(),
			l: true,
			m: rand::random(),
			n: rand_ch,
			v: vec![rand::random(), rand::random(), rand::random()],
			o: None,
		};

		// test it
		ser_helper(ser_out)?;

		// test with ()
		ser_helper(())?;
		// test with a tuple
		ser_helper((rand::random::<u32>(), rand::random::<i128>()))?;

		// test with a string
		ser_helper(("hi there".to_string(), 123))?;

		// test an array
		let x = [3u8; 8];
		ser_helper(x)?;

		// test an error
		let ser_out = SerErr { exp: 100, empty: 0 };
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_err());

		// test with the values that do not generate an error
		let ser_out = SerErr { exp: 99, empty: 0 };
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_ok());

		// generate an error again
		let ser_out = SerErr { exp: 99, empty: 1 };
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_err());

		// test a vec of strings
		let v = vec!["test1".to_string(), "a".to_string(), "okokok".to_string()];
		ser_helper(v)?;

		// test a ref to a string (read is an error beacuse we can't return a reference
		// from read).
		let s = "abc".to_string();
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &&s)?;
		let s1: Result<&String, Error> = deserialize(&mut &v[..]);
		assert!(s1.is_err());

		Ok(())
	}
}
