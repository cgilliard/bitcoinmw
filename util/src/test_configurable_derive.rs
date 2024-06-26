// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[cfg(test)]
mod test {
	use bmw_conf2::*;
	use bmw_derive::*;
	use bmw_err::*;
	use bmw_log::*;

	debug!();

	#[derive(Configurable)]
	struct MyConfig2 {
		#[required]
		v1: u8,
		pub v2: u8,
		pub(crate) v3: u16,
	}

	impl Default for MyConfig2 {
		fn default() -> Self {
			Self {
				v1: 0,
				v2: 3,
				v3: 100,
			}
		}
	}

	#[test]
	fn test_derive_configurable1() -> Result<(), Error> {
		info!("testing derive configurable")?;

		let my = config!(MyConfig2, MyConfig2_Options, vec![V1(10)])?;
		assert_eq!(my.v1, 10);
		assert_eq!(my.v2, 3);
		assert_eq!(my.v3, 100);

		let my = config!(MyConfig2, MyConfig2_Options, vec![V1(10), V2(20)])?;
		assert_eq!(my.v1, 10);
		assert_eq!(my.v2, 20);
		assert_eq!(my.v3, 100);

		let my = config!(MyConfig2, MyConfig2_Options, vec![V1(10), V3(20)])?;
		assert_eq!(my.v1, 10);
		assert_eq!(my.v2, 3);
		assert_eq!(my.v3, 20);

		assert!(config!(MyConfig2, MyConfig2_Options, vec![V2(10), V3(0)]).is_err());
		Ok(())
	}

	#[derive(Configurable)]
	struct EvhConfig {
		threads: u8,
		#[required]
		port: u16,
		slab_count: u64,
		timeout: u128,
		blob: usize,
		x32: u32,
		server_name: String,
		debug: bool,
		test_vec: Vec<u32>,
		test_vec2: Vec<String>,
		other_ports: Vec<u16>,
		default_not_empty: Vec<u8>,
		header: (String, String),
		headers: Vec<(String, String)>,
	}

	impl Default for EvhConfig {
		fn default() -> Self {
			let threads = 1;
			let port = 8080;
			let slab_count = 100;
			let timeout = 100;
			let blob = 0;
			let x32 = 7;
			let server_name = "myname".to_string();
			let debug = false;
			let test_vec = vec![];
			let test_vec2 = vec![];
			let other_ports = vec![];
			let default_not_empty = vec![1, 2, 3];
			let header = ("".to_string(), "".to_string());
			let headers = vec![];
			Self {
				threads,
				port,
				slab_count,
				timeout,
				blob,
				x32,
				server_name,
				debug,
				test_vec,
				test_vec2,
				other_ports,
				default_not_empty,
				header,
				headers,
			}
		}
	}

	trait Evh {
		fn config(&mut self) -> &mut EvhConfig;
	}

	struct EvhImpl {
		config: EvhConfig,
	}

	impl EvhImpl {
		fn new(config: EvhConfig) -> Self {
			Self { config }
		}
	}

	impl Evh for EvhImpl {
		fn config(&mut self) -> &mut EvhConfig {
			&mut self.config
		}
	}

	macro_rules! evh {
		($($config:tt)*) => {{
			use EvhConfig_Options::*;
                        let options: Vec<EvhConfig_Options> = vec![$($config)*];

			match config!(EvhConfig, EvhConfig_Options, options) {
				Ok(config) => {
                                        let ret: Box<dyn Evh> = Box::new(EvhImpl::new(config));
                                        Ok(ret)
                                },
				Err(e) => Err(err!(ErrKind::Configuration, "config error: {}", e)),
			}
		}};
	}

	#[test]
	fn test_derive_configurable_evh() -> Result<(), Error> {
		let mut evh = evh!(Threads(10), Port(8081), ServerName("abc"), Debug(true))?;

		info!("evh.config.port={}", evh.config().port)?;

		assert_eq!(evh.config().port, 8081);

		evh.config().port = 8082;
		assert_eq!(evh.config().port, 8082);
		assert_eq!(evh.config().server_name, "abc".to_string());
		assert_eq!(evh.config().debug, true);

		let mut evh = evh!(
			Port(1234),
			X32(u32::MAX),
			Timeout(1_000_000_000_000),
			TestVec(7u32),
			TestVec(3u32)
		)?;

		assert_eq!(evh.config().port, 1234);
		assert_eq!(evh.config().x32, u32::MAX);
		assert_eq!(evh.config().timeout, 1_000_000_000_000);
		assert_eq!(evh.config().slab_count, 100);
		assert_eq!(evh.config().server_name, "myname".to_string());
		assert_eq!(evh.config().debug, false);
		assert_eq!(evh.config().test_vec, vec![7u32, 3u32]);

		let mut evh = evh!(
			Port(1234),
			TestVec2("hi"),
			TestVec2("there"),
			TestVec2("next"),
			OtherPorts(90),
			OtherPorts(100),
			OtherPorts(110),
		)?;

		assert_eq!(
			evh.config().test_vec2,
			vec!["hi".to_string(), "there".to_string(), "next".to_string()]
		);

		assert_eq!(evh.config().other_ports, vec![90, 100, 110]);

		let mut evh = evh!(Port(5555),)?;

		assert_eq!(evh.config().default_not_empty, vec![1, 2, 3]);

		let mut evh = evh!(Port(5678), DefaultNotEmpty(8))?;

		// note that the 8 is appended to the default. That might not be what users expect,
		// but not a lot of use cases that I can think for having these kind of non-empty
		// default Vecs so we'll not implement a delete for now. Need to document this
		// though.
		assert_eq!(evh.config().default_not_empty, vec![1, 2, 3, 8]);

		let mut evh = evh!(Port(4444), Header(("abc", "def")),)?;

		assert_eq!(evh.config().header, ("abc".to_string(), "def".to_string()));

		let mut evh = evh!(
			Port(6666),
			Headers(("xyz", "ghi")),
			Headers(("zzz", "aaab")),
			Headers(("zzz2", "aaa2")),
		)?;

		assert_eq!(
			evh.config().headers,
			vec![
				("xyz".to_string(), "ghi".to_string()),
				("zzz".to_string(), "aaab".to_string()),
				("zzz2".to_string(), "aaa2".to_string())
			]
		);

		Ok(())
	}

	#[derive(Configurable, PartialEq, Debug)]
	pub struct Empty {}

	impl Default for Empty {
		fn default() -> Self {
			Self {}
		}
	}

	#[derive(Configurable, PartialEq, Debug)]
	struct ConfigAll {
		a1: u8,
		b1: u16,
		c1: u32,
		d1: u64,
		e1: u128,
		f1: usize,
		g1: String,
		h1: bool,
		i1: (String, String),
		j1: Vec<u8>,
		k1: Vec<u16>,
		l1: Vec<u32>,
		m1: Vec<u64>,
		n1: Vec<u128>,
		o1: Vec<usize>,
		p1: Vec<String>,
		q1: Vec<bool>,
		r1: Vec<(String, String)>,
	}

	impl Default for ConfigAll {
		fn default() -> Self {
			let a1 = 1;
			let b1 = 2;
			let c1 = 3;
			let d1 = 4;
			let e1 = 5;
			let f1 = 6;
			let g1 = "default".to_string();
			let h1 = false;
			let i1 = ("def1".to_string(), "def2".to_string());
			let j1 = vec![];
			let k1 = vec![];
			let l1 = vec![];
			let m1 = vec![];
			let n1 = vec![];
			let o1 = vec![];
			let p1 = vec![];
			let q1 = vec![];
			let r1 = vec![];

			Self {
				a1,
				b1,
				c1,
				d1,
				e1,
				f1,
				g1,
				h1,
				i1,
				j1,
				k1,
				l1,
				m1,
				n1,
				o1,
				p1,
				q1,
				r1,
			}
		}
	}

	#[test]
	fn test_derive_configuration_exhaustive() -> Result<(), Error> {
		let empty = config!(Empty, Empty_Options, vec![])?;
		assert_eq!(empty, Empty {});

		let all = config!(ConfigAll, ConfigAll_Options, vec![])?;
		assert_eq!(all, ConfigAll::default());

		let all = config!(ConfigAll, ConfigAll_Options, vec![A1(0)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.a1, 0);

		let all = config!(ConfigAll, ConfigAll_Options, vec![B1(0)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.b1, 0);

		let all = config!(ConfigAll, ConfigAll_Options, vec![C1(0)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.c1, 0);

		let all = config!(ConfigAll, ConfigAll_Options, vec![D1(0)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.d1, 0);

		let all = config!(ConfigAll, ConfigAll_Options, vec![E1(0)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.e1, 0);

		let all = config!(ConfigAll, ConfigAll_Options, vec![F1(0)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.f1, 0);

		let all = config!(ConfigAll, ConfigAll_Options, vec![G1("nondefault")])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.g1, "nondefault".to_string());

		let all = config!(ConfigAll, ConfigAll_Options, vec![H1(true)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.h1, true);

		let all = config!(ConfigAll, ConfigAll_Options, vec![I1(("a", "b"))])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.i1, ("a".to_string(), "b".to_string()));

		let all = config!(ConfigAll, ConfigAll_Options, vec![J1(0)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.j1, vec![0]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![J1(0), J1(1)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.j1, vec![0, 1]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![K1(0)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.k1, vec![0]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![K1(0), K1(1)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.k1, vec![0, 1]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![L1(0)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.l1, vec![0]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![L1(0), L1(1)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.l1, vec![0, 1]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![M1(0)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.m1, vec![0]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![M1(0), M1(1)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.m1, vec![0, 1]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![N1(0)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.n1, vec![0]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![N1(0), N1(1)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.n1, vec![0, 1]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![O1(0)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.o1, vec![0]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![O1(0), O1(1)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.o1, vec![0, 1]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![P1("1")])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.p1, vec!["1".to_string()]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![P1("1"), P1("2")])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.p1, vec!["1".to_string(), "2".to_string()]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![Q1(false)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.q1, vec![false]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![Q1(true), Q1(false)])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.q1, vec![true, false]);

		let all = config!(ConfigAll, ConfigAll_Options, vec![R1(("x", "y"))])?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(all.r1, vec![("x".to_string(), "y".to_string())]);

		let all = config!(
			ConfigAll,
			ConfigAll_Options,
			vec![R1(("x", "y")), R1(("a", "b"))]
		)?;
		assert_ne!(all, ConfigAll::default());
		assert_eq!(
			all.r1,
			vec![
				("x".to_string(), "y".to_string()),
				("a".to_string(), "b".to_string())
			]
		);

		Ok(())
	}

	#[derive(Configurable, PartialEq, Debug)]
	pub struct Required1 {
		#[required]
		r1: u8,
		r2: u16,
		r3: String,
	}

	#[derive(Configurable, PartialEq, Debug)]
	pub struct Required2 {
		r1: u8,
		#[required]
		pub r2: u16,
		pub(crate) r3: String,
	}

	#[derive(Configurable, PartialEq, Debug)]
	pub struct Required3 {
		pub(crate) r1: u8,
		pub r2: u16,
		#[required]
		pub(crate) r3: String,
	}

	impl Default for Required1 {
		fn default() -> Self {
			Self {
				r1: 0,
				r2: 0,
				r3: "".to_string(),
			}
		}
	}

	impl Default for Required2 {
		fn default() -> Self {
			Self {
				r1: 0,
				r2: 0,
				r3: "".to_string(),
			}
		}
	}

	impl Default for Required3 {
		fn default() -> Self {
			Self {
				r1: 0,
				r2: 0,
				r3: "".to_string(),
			}
		}
	}

	#[test]
	fn test_derive_configuration_required() -> Result<(), Error> {
		assert!(config!(Required1, Required1_Options, vec![]).is_err());
		assert_eq!(
			config!(Required1, Required1_Options, vec![R1(0)])?,
			Required1::default()
		);

		assert!(config!(Required2, Required2_Options, vec![]).is_err());
		assert_eq!(
			config!(Required2, Required2_Options, vec![R2(0)])?,
			Required2::default()
		);

		assert!(config!(Required3, Required3_Options, vec![]).is_err());
		assert_eq!(
			config!(Required3, Required3_Options, vec![R3("")])?,
			Required3::default()
		);

		Ok(())
	}
}
