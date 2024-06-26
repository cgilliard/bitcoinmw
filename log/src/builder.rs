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

use crate::types::LogImpl;
use crate::{Log, LogBuilder, LogConfig2_Options};
use bmw_err::Error;

impl LogBuilder {
	/// Build a logger based based on the specified configuration. This should generally be
	/// done by calling the [`crate::logger`] macro.
	pub fn build_log(
		configs: Vec<LogConfig2_Options>,
	) -> Result<Box<dyn Log + Send + Sync>, Error> {
		Ok(Box::new(LogImpl::new(configs)?))
	}
}
