// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw and Grin Developers
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

//! Build hooks to spit out version+build time info

use built;

use std::env;
use std::path::Path;

fn main() {
	let out_dir_path = format!("{}{}", env::var("OUT_DIR").unwrap(), "/built.rs");
	// don't fail the build if something's missing, may just be cargo release
	let _ = built::write_built_file_with_opts(
		Some(Path::new(env!("CARGO_MANIFEST_DIR"))),
		Path::new(&out_dir_path),
	);
}
