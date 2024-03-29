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

use bmw_log::*;

info!();

/// Macro to get a [`crate::Lock`]. Internally, the parameter passed in is wrapped in
/// an `Arc<Rwlock<T>>` wrapper that can be used to obtain read/write locks around any
/// data structure.
///
/// # Examples
///
///```
/// use bmw_err::*;
/// use bmw_util::*;
/// use std::time::Duration;
/// use std::thread::{sleep, spawn};
///
/// #[derive(Debug, PartialEq)]
/// struct MyStruct {
///     id: u128,
///     name: String,
/// }
///
/// impl MyStruct {
///     fn new(id: u128, name: String) -> Self {
///         Self { id, name }
///     }
/// }
///
/// fn main() -> Result<(), Error> {
///     let v = MyStruct::new(1234, "joe".to_string());
///     let mut vlock = lock!(v)?;
///     let vlock_clone = vlock.clone();
///
///     spawn(move || -> Result<(), Error> {
///         let mut x = vlock.wlock()?;
///         assert_eq!((**(x.guard()?)).id, 1234);
///         sleep(Duration::from_millis(3000));
///         (**(x.guard()?)).id = 4321;
///         Ok(())
///     });
///
///     sleep(Duration::from_millis(1000));
///     let x = vlock_clone.rlock()?;
///     assert_eq!((**(x.guard()?)).id, 4321);
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! lock {
	($value:expr) => {{
		bmw_util::UtilBuilder::build_lock($value)
	}};
}

/// The same as [`crate::lock!`] except that the value returned is in a `Box<dyn LockBox<T>>` structure.
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_util::*;
/// use std::time::Duration;
/// use std::thread::{sleep, spawn};
///
/// #[derive(Debug, PartialEq)]
/// struct MyStruct {
///     id: u128,
///     name: String,
/// }   
///
/// impl MyStruct {
///     fn new(id: u128, name: String) -> Self {
///         Self { id, name }
///     }
/// }
///
/// fn main() -> Result<(), Error> {
///     let v = MyStruct::new(1234, "joe".to_string());
///     let mut vlock = lock_box!(v)?;
///     let vlock_clone = vlock.clone();
///         
///     spawn(move || -> Result<(), Error> {
///         let mut x = vlock.wlock()?;
///         assert_eq!((**(x.guard()?)).id, 1234);
///         sleep(Duration::from_millis(3000));
///         (**(x.guard()?)).id = 4321;
///         Ok(())
///     });
///
///     sleep(Duration::from_millis(1000));
///     let x = vlock_clone.rlock()?;
///     assert_eq!((**(x.guard()?)).id, 4321);
///     
///     Ok(())  
/// }   
///```
#[macro_export]
macro_rules! lock_box {
	($value:expr) => {{
		bmw_util::UtilBuilder::build_lock_box($value)
	}};
}

/// macro to call wlock and guard function on a [`crate::LockBox`] at the same time. Note that this only allows
/// a single access to the variable. If more than one operation needs to be done, this macro
/// should not be used.
#[macro_export]
macro_rules! wlock {
	($value:expr) => {
		**($value.wlock()?.guard()?)
	};
}

/// macro to call rlock and guard function on a [`crate::LockBox`] at the same time. Note that this only allows
/// a single access to the variable. If more than one operation needs to be done, this macro
/// should not be used.
#[macro_export]
macro_rules! rlock {
	($value:expr) => {
		**($value.rlock()?.guard()?)
	};
}

/// The [`crate::global_slab_allocator`] macro initializes the global thread local slab allocator
/// for the thread that it is executed in.
///
/// # Input Parameters
/// * SlabSize([`prim@usize`]) (optional) - the size in bytes of the slabs for this slab allocator.
///                                if not specified, the default value of 256 is used.
///
/// * SlabCount([`prim@usize`]) (optional) - the number of slabs to allocate to the global slab
///                                 allocator. If not specified, the default value of
///                                 40,960 is used.
///
/// # Return
/// Return Ok(()) on success or [`bmw_err::Error`] on failure.
///
/// # Errors
///
/// * [`bmw_err::ErrKind::Configuration`] - Is returned if a
///                                           ConfigOption other than
///                                           ConfigOption::SlabSize or
///                                           ConfigOption::SlabCount is
///                                           specified.
///
/// * [`bmw_err::ErrKind::IllegalState`] - Is returned if the global thread local
///                                          slab allocator has already been initialized
///                                          for the thread that executes the macro. This
///                                          can happen if the macro is called more than once
///                                          or if a data structure that uses the global
///                                          slab allocator is initialized and in turn initializes
///                                          the global slab allocator with default values.
///
/// * [`bmw_err::ErrKind::IllegalArgument`] - Is returned if the SlabSize is 0 or the SlabCount
///                                             is 0.
///
/// # Examples
///```
/// use bmw_util::*;
/// use bmw_err::Error;
///
/// fn main() -> Result<(), Error> {
///     global_slab_allocator!(SlabSize(128), SlabCount(1_000))?;
///
///     // this will use the global slab allocator since we don't specify SlabSize or SlabCount.
///     let hashtable: Box<dyn Hashtable<u32, u32>> = hashtable_box!()?;
///
///     // ...
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! global_slab_allocator {
( $( $config:expr ),* ) => {{
            #[allow(unused_imports)]
            use bmw_conf::ConfigOption::*;
            let mut config = bmw_util::SlabAllocatorConfig::default();
            let mut error: Option<String> = None;
            let mut slab_size_specified = false;
            let mut slab_count_specified = false;

            // compiler sees macro as not used if it's not used in one part of the code
            // these lines make the warnings go away
            if config.slab_size == 0 { config.slab_size = 0; }
            if slab_count_specified { slab_count_specified = false; }
            if slab_size_specified { slab_size_specified = false; }
            if slab_count_specified {}
            if slab_size_specified {}
            if error.is_some() { error = None; }

            $(
                match $config {
                    bmw_conf::ConfigOption::SlabSize(slab_size) => {
                        config.slab_size = slab_size;

                        if slab_size_specified {
                            error = Some("SlabSize was specified more than once!".to_string());
                        }
                        slab_size_specified = true;
                        if slab_size_specified {}

                    },
                    bmw_conf::ConfigOption::SlabCount(slab_count) => {
                        config.slab_count = slab_count;

                        if slab_count_specified {
                            error = Some("SlabCount was specified more than once!".to_string());
                        }

                        slab_count_specified = true;
                        if slab_count_specified {}
                    },
                    _ => {
                        error = Some(format!("'{:?}' is not allowed for hashset", $config));
                    }
                }
            )*

            match error {
                Some(error) => Err(bmw_err::err!(ErrKind::Configuration, error)),
                None => {
                        bmw_util::GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<(), Error> {
                        unsafe {
                                f.get().as_mut().unwrap().init(config)?;
                                Ok(())
                        }
                        })
                }
            }
        }
    }
}

/// The `slab_allocator` macro initializes a slab allocator with the specified parameters.
/// It takes the following parameters:
///
/// * SlabSize([`prim@usize`]) (optional) - the size in bytes of the slabs for this slab allocator.
///                                if not specified, the default value of 256 is used.
///
/// * SlabCount([`prim@usize`]) (optional) - the number of slabs to allocate to this slab
///                                 allocator. If not specified, the default value of
///                                 40,960 is used.
///
/// # Return
/// Return `Ok(Rc<RefCell<dyn SlabAllocator>>)` on success or [`bmw_err::Error`] on failure.
///
/// # Errors
///
/// * [`bmw_err::ErrKind::Configuration`] - Is returned if a
///                                           ConfigOption other than
///                                           ConfigOption::SlabSize or
///                                           ConfigOption::SlabCount is
///                                           specified.
///
/// * [`bmw_err::ErrKind::IllegalArgument`] - Is returned if the SlabSize is 0 or the SlabCount
///                                             is 0.
///
/// # Examples
///```
/// use bmw_util::*;
/// use bmw_err::Error;
///
/// fn main() -> Result<(), Error> {
///     let mut slabs = slab_allocator!(SlabSize(128), SlabCount(5))?;
///
///     let id = {
///         // allocate a slab
///         let mut slab = slabs.allocate()?;
///
///         // get an immutable reference to the slab
///         let slab_ref = slab.get();
///
///         let mut count = 0;
///         for v in slab_ref {
///             count += 1;
///         }
///         // slab should be SlabSize (128)
///         assert_eq!(count, 128);
///
///         // get a mutable reference to the slab
///         let mut slab_ref = slab.get_mut();
///
///         let mut count = 0;
///         for i in 0..slab_ref.len() {
///             // we can write to this slab_ref
///             slab_ref[i] = 10;
///             count += 1;
///         }
///         // slab should be SlabSize (128)
///         assert_eq!(count, 128);
///
///
///         slab.id()
///     };
///
///     
///
///     // free the slab
///     slabs.free(id)?;
///
///     // allocate all 5 slabs in this slab allocator
///     let mut slab_vec = vec![];
///     for _ in 0..5 {
///         let slab = slabs.allocate()?;
///         slab_vec.push(slab.id());
///     }
///
///     // no more slabs
///     assert!(slabs.allocate().is_err());
///
///     // free the first slabs in our vec
///     slabs.free(slab_vec[0]);
///
///     // now we can allocate again
///     let slab = slabs.allocate()?;
///
///     // ...
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! slab_allocator {
	($($config:tt)*) => {{
		use bmw_conf::config;
		#[allow(unused_imports)]
		use bmw_conf::ConfigOptionName as CN;
		use bmw_err::*;
		use bmw_util::{SlabAllocatorConfig, UtilBuilder};
		let mut slab_config = SlabAllocatorConfig::default();
		let config = config!($($config)*);
	        match config.check_config(vec![CN::SlabSize, CN::SlabCount], vec![]) {
                        Ok(_) => {

		                slab_config.slab_size = config.get_or_usize(&CN::SlabSize, slab_config.slab_size);
		                slab_config.slab_count = config.get_or_usize(&CN::SlabCount, slab_config.slab_count);

		                let mut slabs = UtilBuilder::build_sync_slabs();
		                match slabs.init(slab_config) {
			                Ok(_) => Ok(slabs),
			                Err(e) => {
				                        let text = format!("Could not init slabs due to: {}", e.to_string());
					                Err(err!(ErrKind::IllegalState, text))
			                }
		                }
                        }
                        Err(e) => {
                                let text = format!("Could not configure slabs due to: {}", e.to_string());
                                Err(err!(ErrKind::Configuration, text))
                        }
                }
	}};
}

/// The pattern macro builds a [`crate::Pattern`] which is used by the [`crate::SearchTrie`].
/// The pattern macro takes the following parameters:
///
/// * Regex([`std::string::String`]) (required) - The regular expression to use for matching (note this is not a
///                                      full regular expression. Only some parts of regular expressions
///                                      are implemented like wildcards and carets). See [`crate::Pattern`]
///                                      for full details.
/// * PatternId([`prim@usize`]) (required) - The id for this pattern. This id is returned in the
///                                      [`crate::Match`] array if this match occurs when the
///                                      [`crate::SearchTrie::tmatch`] function is called.
/// * IsMultiLine([`bool`]) (optional) - If true is specified this pattern is a multi-line pattern meaning
///                                      that wildcards can cross newlines. Otherwise newlines are not
///                                      allowed in wildcard matches. If not specified this is
///                                      true.
/// * IsTerminationPattern([`bool`]) (optional) - If true, this is a termination pattern meaning that if it is
///                                      found, when the [`crate::SearchTrie::tmatch`] function is called,
///                                      matching will terminate and the matches found up to that point in
///                                      the text will be returned. If not specified this is false.
/// * IsCaseSensitive([`bool`]) (optional) - If true only case sensitive matches are returned for this
///                                      pattern. Otherwise, case-insensitive matches are also returned.
///                                      If not specified, this is false.
///
/// # Return
/// Returns `Ok(Pattern)` on success and on error a [`bmw_err::Error`] is returned.
///
/// # Errors
/// * [`bmw_err::ErrKind::Configuration`] - If any other options are specified or if there are
/// duplicates.
/// * [`bmw_err::ErrKind::Configuration`] - If a Regex or PatternId is not specified.
/// * [`bmw_err::ErrKind::Configuration`] - If both IsCaseSensitive and IsTerminationPattern are
/// specified. (not currently supported)
///
/// # Examples
///```
/// use bmw_util::*;
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///         // build a suffix tree with three patterns
///         let mut search_trie = search_trie!(
///                 vec![
///                         // create a pattern that's multi-line and case sensitive
///                         pattern!(
///                             Regex("p1".to_string()),
///                             PatternId(0),
///                             IsTerminationPattern(false),
///                             IsMultiLine(true),
///                             IsCaseSensitive(true)
///                         )?,
///                         // create a pattern that is a termination pattern
///                         pattern!(
///                             Regex("p2".to_string()),
///                             PatternId(1),
///                             IsTerminationPattern(true),
///                             IsCaseSensitive(false)
///                         )?,
///                         // use defaults (case insensitive, not termination and multiline true)
///                         pattern!(Regex("p3".to_string()), PatternId(2))?
///                 ],
///                 TerminationLength(1_000),
///                 MaxWildCardLength(100)
///         )?;
///
///         // create a matches array for the suffix tree to return matches in
///         let mut matches = [tmatch!()?; 10];
///
///         // run the match for the input text b"p1p2".
///         let count = search_trie.tmatch(b"p1p2", &mut matches)?;
///
///         // assert that two matches were returned "p1" and "p2"
///         // and that their start/end/id is correct.
///         info!("count={}", count)?;
///         assert_eq!(count, 2);
///         assert_eq!(matches[1].id(), 0);
///         assert_eq!(matches[1].start(), 0);
///         assert_eq!(matches[1].end(), 2);
///         assert_eq!(matches[0].id(), 1);
///         assert_eq!(matches[0].start(), 2);
///         assert_eq!(matches[0].end(), 4);
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! pattern {
	( $( $pattern_items:tt)* ) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($pattern_items)*];
                bmw_util::UtilBuilder::build_pattern(v)
	}};
}

/// Create a default instance of [`crate::Match`].
#[macro_export]
macro_rules! tmatch {
        ( $( $match_items:tt)* ) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($match_items)*];
                bmw_util::UtilBuilder::build_match(v)
        }};
}

/// The `search_trie` macro builds a [`crate::SearchTrie`] which can be used to match multiple
/// patterns for a given text in a performant way.
/// # Input Parameters
///
/// * `List<Pattern>`            (required) - The list of [`crate::Pattern`]s that this [`crate::SearchTrie`]
///                                         will use to match.
/// * TerminationLength ([`prim@usize`]) (optional) - The length in bytes at which matching will terminate.
/// * MaxWildCardLength ([`prim@usize`]) (optional) - The maximum length in bytes of a wild card match.
///
/// # Return
/// Returns `Ok(impl SearchTrie + Send + Sync)` on success and on error a [`bmw_err::Error`] is returned.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalArgument`] - If one of the regular expressions is invalid.
///                                             or the length of the patterns list is 0.
///
/// # Examples
///
///```
/// use bmw_util::*;
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///         // build a suffix tree with three patterns
///         let mut search_trie = search_trie!(
///                 vec![
///                         pattern!(Regex("p1".to_string()), PatternId(0))?,
///                         pattern!(Regex("p2".to_string()), PatternId(1))?,
///                         pattern!(Regex("p3".to_string()), PatternId(2))?
///                 ],
///                 TerminationLength(1_000),
///                 MaxWildCardLength(100)
///         )?;
///
///         // create a matches array for the suffix tree to return matches in
///         let mut matches = [tmatch!()?; 10];
///
///         // run the match for the input text b"p1p2".
///         let count = search_trie.tmatch(b"p1p2", &mut matches)?;
///
///         // assert that two matches were returned "p1" and "p2"
///         // and that their start/end/id is correct.
///         info!("count={}", count)?;
///         assert_eq!(count, 2);
///         assert_eq!(matches[0].id(), 0);
///         assert_eq!(matches[0].start(), 0);
///         assert_eq!(matches[0].end(), 2);
///         assert_eq!(matches[1].id(), 1);
///         assert_eq!(matches[1].start(), 2);
///         assert_eq!(matches[1].end(), 4);
///
///         Ok(())
/// }
///```
///
/// Wild card match
///
///```
/// use bmw_util::*;
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///         // build a suffix tree with a wild card
///         let mut search_trie = search_trie!(
///                 vec![
///                         pattern!(Regex("p1".to_string()), PatternId(0))?,
///                         pattern!(Regex("p2.*test".to_string()), PatternId(1))?,
///                         pattern!(Regex("p3".to_string()), PatternId(2))?
///                 ],
///                 TerminationLength(1_000),
///                 MaxWildCardLength(100)
///         )?;
///
///         // create a matches array for the suffix tree to return matches in
///         let mut matches = [UtilBuilder::build_match(vec![])?; 10];
///
///         // run the match for the input text b"p1p2". Only "p1" matches this time.
///         let count = search_trie.tmatch(b"p1p2", &mut matches)?;
///         assert_eq!(count, 1);
///
///         // run the match for the input text b"p1p2xxxxxxtest1". Now the wildcard
///         // match succeeds to two matches are returned.
///         let count = search_trie.tmatch(b"p1p2xxxxxxtest1", &mut matches)?;
///         assert_eq!(count, 2);
///
///         Ok(())
/// }
///```
///
/// Single character wild card
///
///```
/// use bmw_util::*;
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///         // build a suffix tree with a wild card
///         let mut search_trie = search_trie!(
///                 vec![
///                         pattern!(Regex("p1".to_string()), PatternId(0))?,
///                         pattern!(Regex("p2.test".to_string()), PatternId(1))?,
///                         pattern!(Regex("p3".to_string()), PatternId(2))?
///                 ],
///                 TerminationLength(1_000),
///                 MaxWildCardLength(100)
///         )?;
///
///         // create a matches array for the suffix tree to return matches in
///         let mut matches = [tmatch!()?; 10];
///
///         // run the match for the input text b"p1p2". Only "p1" matches this time.
///         let count = search_trie.tmatch(b"p1p2", &mut matches)?;
///         assert_eq!(count, 1);
///
///         // run the match for the input text b"p1p2xxxxxxtest1". Now the wildcard
///         // match doesn't succeed because it's a single char match. One match is returned.
///         let count = search_trie.tmatch(b"p1p2xxxxxxtest1", &mut matches)?;
///         assert_eq!(count, 1);
///
///         // run it with a single char and see that it matches pattern two.
///         let count = search_trie.tmatch(b"p1p2xtestx", &mut matches)?;
///         assert_eq!(count, 2);
///
///         Ok(())
/// }
///```
///
/// Match at the beginning of the text
///
///```
/// use bmw_util::*;
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {      
///         // build a suffix tree with a wild card
///         let mut search_trie = search_trie!(
///                 vec![
///                         pattern!(Regex("p1".to_string()), PatternId(0))?,
///                         pattern!(Regex("^p2".to_string()), PatternId(2))?
///                 ],
///                 TerminationLength(1_000),
///                 MaxWildCardLength(100)
///         )?;
///
///         // create a matches array for the suffix tree to return matches in
///         let mut matches = [tmatch!()?; 10];
///
///         // run the match for the input text b"p1p2". Only "p1" matches this time
///         // because p2 is not at the start
///         let count = search_trie.tmatch(b"p1p2", &mut matches)?;
///         assert_eq!(count, 1);
///
///         // since p2 is at the beginning, both match
///         let count = search_trie.tmatch(b"p2p1", &mut matches)?;
///         assert_eq!(count, 2);
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! search_trie {
	( $patterns:expr, $($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_search_trie($patterns, v)
        }};
}

/// The `search_trie_box` macro is the `boxed` version of [`crate::search_trie!`]. This macro builds a
/// [`crate::SearchTrie`] which can be used to match multiple patterns for a given text in a performant way.
/// # Input Parameters
///
/// * `List<Pattern>` (required) - The list of [`crate::Pattern`]s that this [`crate::SearchTrie`]
/// will use to match.
/// * TerminationLength ([`prim@usize`]) (optional) - The length in bytes at which matching will terminate.
/// * MaxWildCardLength ([`prim@usize`]) (optional) - The maximum length in bytes of a wild card match.
///
/// # Return
/// Returns `Ok(Box<dyn SearchTrie + Send + Sync>)` on success and on error a [`bmw_err::Error`] is returned.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalArgument`] - If one of the regular expressions is invalid.
///                                             or the length of the patterns list is 0.
///
/// # Examples
///
///```
/// use bmw_util::*;
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///         // build a suffix tree with three patterns
///         let mut search_trie = search_trie_box!(
///                 vec![
///                         pattern!(Regex("p1".to_string()), PatternId(0))?,
///                         pattern!(Regex("p2".to_string()), PatternId(1))?,
///                         pattern!(Regex("p3".to_string()), PatternId(2))?
///                 ],
///                 TerminationLength(1_000),
///                 MaxWildCardLength(100)
///         )?;
///
///         // create a matches array for the suffix tree to return matches in
///         let mut matches = [tmatch!()?; 10];
///
///         // run the match for the input text b"p1p2".
///         let count = search_trie.tmatch(b"p1p2", &mut matches)?;
///
///         // assert that two matches were returned "p1" and "p2"
///         // and that their start/end/id is correct.
///         info!("count={}", count)?;
///         assert_eq!(count, 2);
///         assert_eq!(matches[0].id(), 0);
///         assert_eq!(matches[0].start(), 0);
///         assert_eq!(matches[0].end(), 2);
///         assert_eq!(matches[1].id(), 1);
///         assert_eq!(matches[1].start(), 2);
///         assert_eq!(matches[1].end(), 4);
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! search_trie_box {
        ( $patterns:expr, $($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_search_trie_box($patterns, v)
        }};
}

/// The [`crate::hashtable`] macro builds a [`crate::Hashtable`] with the specified configuration.
///
/// # Input Parameters
/// * MaxEntries ([`prim@usize`]) (optional) - The maximum number of entries that can be in this
/// hashtable at any given time. The default value is 1_000.
/// * MaxLoadFactor ([`prim@usize`]) (optional) - The maximum load factor of the hashtable. The hashtable
/// is an array based hashtable and it has a fixed size. Once the load factor is reached, insertions will
/// return an error. The hashtable uses linear probing to handle collisions. The max_load_factor makes sure
/// no additional insertions occur at a given ratio of entries to capacity in the array. Note that
/// MaxEntries can always be inserted, it's the capacity of the array that becomes larger as this ratio
/// goes down. So if 100 MaxEntries are specified and the MaxLoadFactor is 0.5, a 200 slot array
/// will be used and 100 entries will be allowed. The default MaxLoadFactor is 0.7.
/// * GlobalSlabAllocator ([`bool`]) (optional) - If true, the [`crate::global_slab_allocator`] is
/// used instead of using an internally built slab allocator. The global slab allocator is
/// thread_local and the returned value cannot be passed to other threads. The default value is
/// true.
/// * SlabSize ([`prim@usize`]) (optional) - The size of slabs for the [`crate::SlabAllocator`] associated
/// with this [`crate::Hashtable`]. This option is only allowed if GlobalSlabAllocator is false.
/// * SlabCount ([`prim@usize`]) (optional) - The count of slabs. This option is only allowed if
/// GlobalSlabAllocator is false.
///
/// # Returns
///
/// A Ok(`impl Hashtable<K, V>`) on success or a [`bmw_err::Error`] on failure.
///
/// # Errors
///
/// * [`bmw_err::ErrKind::Configuration`] - If any values are specified other than the allowed
/// values mentioned above or if there are any duplicate parameters specified.
/// * [`bmw_err::ErrKind::Configuration`] - If only one of SlabSize and SlabCount are specified.
/// * [`bmw_err::ErrKind::Configuration`] - If GlobalSlabAllocator is true and SlabSize or
/// SlabCount are specified.
/// * [`bmw_err::ErrKind::Configuration`] - If GlobalSlabAllocator is false and SlabSize or
/// SlabCount are not specified.
/// * [`bmw_err::ErrKind::IllegalArgument`] - If the parameters specified for the SlabSize or
/// SlabCount are not valid. See [`crate::SlabAllocator`].
///
/// # Examples
///```                  
/// use bmw_util::*;
/// use bmw_log::*;             
/// use bmw_err::*;             
///
/// fn main() -> Result<(), Error> {
///         // create a hashtable with the specified parameters
///         let mut hashtable = hashtable!(
///                 MaxEntries(1_000),  
///                 MaxLoadFactor(0.9),                 
///                 GlobalSlabAllocator(false),         
///                 SlabSize(100),      
///                 SlabCount(100)
///         )?;         
///                     
///         // do an insert, rust will figure out what type is being inserted
///         hashtable.insert(&1u32, &10u128)?;
///                     
///         // assert that the entry was inserted
///         assert_eq!(hashtable.get(&1u32)?, Some(10u128));
///
///         // create another hashtable with defaults, this time the global slab allocator will be
///         // used. Since we did not initialize it default values will be used.
///         let mut hashtable = hashtable!()?;
///
///         // do an insert, rust will figure out what type is being inserted
///         hashtable.insert(&1u8, &100u16)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashtable.get(&1u8)?, Some(100u16));
///
///         // assert that this entry is not in the table
///         assert_eq!(hashtable.get(&2u8)?, None);
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! hashtable {
	($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashtable(v)
        }};
}

/// The [`crate::hashtable_box`] macro is the `boxed` version of [`crate::hashtable`]. It builds a
/// [`crate::Hashtable`] with the specified configuration and stores it in a [`std::boxed::Box`]
/// (`Box<dyn Hashtable<K, V>>`).
///
/// # Input Parameters
/// * MaxEntries ([`prim@usize`]) (optional) - The maximum number of entries that can be in this
/// hashtable at any given time. The default value is 1_000.
/// * MaxLoadFactor ([`prim@usize`]) (optional) - The maximum load factor of the hashtable. The hashtable
/// is an array based hashtable and it has a fixed size. Once the load factor is reached, insertions will
/// return an error. The hashtable uses linear probing to handle collisions. The max_load_factor makes sure
/// no additional insertions occur at a given ratio of entries to capacity in the array. Note that
/// MaxEntries can always be inserted, it's the capacity of the array that becomes larger as this ratio
/// goes down. So if 100 MaxEntries are specified and the MaxLoadFactor is 0.5, a 200 slot array
/// will be used and 100 entries will be allowed. The default MaxLoadFactor is 0.7.
/// * GlobalSlabAllocator ([`bool`]) (optional) - If true, the [`crate::global_slab_allocator`] is
/// used instead of using an internally built slab allocator. The global slab allocator is
/// thread_local and the returned value cannot be passed to other threads. The default value is
/// true.
/// * SlabSize ([`prim@usize`]) (optional) - The size of slabs for the [`crate::SlabAllocator`] associated
/// with this [`crate::Hashtable`]. This option is only allowed if GlobalSlabAllocator is false.
/// * SlabCount ([`prim@usize`]) (optional) - The count of slabs. This option is only allowed if
/// GlobalSlabAllocator is false.
///
/// # Returns           
///
/// A Ok(`Box<dyn Hashtable<K, V>>`) on success or a [`bmw_err::Error`] on failure.
///
/// # Errors
///
/// * [`bmw_err::ErrKind::Configuration`] - If any values are specified other than the allowed
/// values mentioned above or if there are any duplicate parameters specified.
/// * [`bmw_err::ErrKind::Configuration`] - If only one of SlabSize and SlabCount are specified.
/// * [`bmw_err::ErrKind::Configuration`] - If GlobalSlabAllocator is true and SlabSize or
/// SlabCount are specified.
/// * [`bmw_err::ErrKind::Configuration`] - If GlobalSlabAllocator is false and SlabSize or
/// SlabCount are not specified.
/// * [`bmw_err::ErrKind::IllegalArgument`] - If the parameters specified for the SlabSize or
/// SlabCount are not valid. See [`crate::SlabAllocator`].
///
/// # Examples
///```                  
/// use bmw_util::*;
/// use bmw_log::*;
/// use bmw_err::*;
///
/// fn main() -> Result<(), Error> {
///         // create a hashtable with the specified parameters
///         let mut hashtable = hashtable_box!(
///                 MaxEntries(1_000),
///                 MaxLoadFactor(0.9),
///                 GlobalSlabAllocator(false),
///                 SlabSize(100),
///                 SlabCount(100)
///         )?;   
///
///         // do an insert, rust will figure out what type is being inserted
///         hashtable.insert(&1u32, &10u128)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashtable.get(&1u32)?, Some(10u128));
///
///         // create another hashtable with defaults, this time the global slab allocator will be
///         // used. Since we did not initialize it default values will be used.
///         let mut hashtable = hashtable_box!()?;
///
///         // do an insert, rust will figure out what type is being inserted
///         hashtable.insert(&1u8, &100u16)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashtable.get(&1u8)?, Some(100u16));
///
///         // assert that this entry is not in the table
///         assert_eq!(hashtable.get(&2u8)?, None);
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! hashtable_box {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashtable_box(v)
        }};
}

/// The [`crate::hashtable_sync`] macro is the `sync` version of [`crate::hashtable`]. It builds a
/// [`crate::Hashtable`] with the specified configuration and returns it with the Send and Sync
/// markers.
/// (`impl Hashtable<K, V> + Send + Sync`).
///
/// # Input Parameters
/// * MaxEntries ([`prim@usize`]) (optional) - The maximum number of entries that can be in this
/// hashtable at any given time. The default value is 1_000.
/// * MaxLoadFactor ([`prim@usize`]) (optional) - The maximum load factor of the hashtable. The hashtable
/// is an array based hashtable and it has a fixed size. Once the load factor is reached, insertions will
/// return an error. The hashtable uses linear probing to handle collisions. The max_load_factor makes sure
/// no additional insertions occur at a given ratio of entries to capacity in the array. Note that
/// MaxEntries can always be inserted, it's the capacity of the array that becomes larger as this ratio
/// goes down. So if 100 MaxEntries are specified and the MaxLoadFactor is 0.5, a 200 slot array
/// will be used and 100 entries will be allowed. The default MaxLoadFactor is 0.7.
/// * GlobalSlabAllocator ([`bool`]) (optional) - If true, the [`crate::global_slab_allocator`] is
/// used instead of using an internally built slab allocator. The global slab allocator is
/// thread_local and the returned value cannot be passed to other threads. The default value is
/// true.
/// * SlabSize ([`prim@usize`]) (optional) - The size of slabs for the [`crate::SlabAllocator`] associated
/// with this [`crate::Hashtable`]. This option is only allowed if GlobalSlabAllocator is false.
/// * SlabCount ([`prim@usize`]) (optional) - The count of slabs. This option is only allowed if
/// GlobalSlabAllocator is false.
///
/// # Returns
///
/// A Ok(`impl Hashtable<K, V> + Send + Sync`) on success or a [`bmw_err::Error`] on failure.
///
/// # Errors
///
/// * [`bmw_err::ErrKind::Configuration`] - If any values are specified other than the allowed
/// values mentioned above or if there are any duplicate parameters specified.
/// * [`bmw_err::ErrKind::Configuration`] - If SlabSize or SlabCount are not specified.
/// * [`bmw_err::ErrKind::Configuration`] - If GlobalSlabAllocator is true (not allowed for sync)
/// * [`bmw_err::ErrKind::IllegalArgument`] - If the parameters specified for the SlabSize or
/// SlabCount are not valid. See [`crate::SlabAllocator`].
///
/// # Examples
///```      
/// use bmw_util::*;
/// use bmw_log::*;
/// use bmw_err::*;
///         
/// fn main() -> Result<(), Error> {
///         // create a hashtable with the specified parameters
///         let mut hashtable = hashtable_sync!(
///                 MaxEntries(1_000),
///                 MaxLoadFactor(0.9),
///                 GlobalSlabAllocator(false),
///                 SlabSize(100),
///                 SlabCount(100)
///         )?;
///             
///         // do an insert, rust will figure out what type is being inserted
///         hashtable.insert(&1u64, &10u8)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashtable.get(&1u64)?, Some(10u8));
///
///         // a hashtable_sync cannot be created with the global slab allocator
///         // at a minimum GlobalSlabAllocator must be false and SlabSize/SlabCount must
///         // be specified
///         let mut hashtable = hashtable_sync!(
///             GlobalSlabAllocator(false),
///             SlabSize(100),
///             SlabCount(200),
///         )?;
///
///         hashtable.insert(&1i32, &10i128)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashtable.get(&1i32)?, Some(10i128));
///
///         // assert that this entry is not in the table
///         assert_eq!(hashtable.get(&2i32)?, None);
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! hashtable_sync {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashtable_sync(v)
        }};
}

/// The [`crate::hashtable_sync_box`] macro is the `sync` and `boxed` version of [`crate::hashtable`].
/// It builds a [`crate::Hashtable`] with the specified configuration and returns it with the Send and Sync
/// markers in a [`std::boxed::Box`].  (`Box<dyn  Hashtable<K, V> + Send + Sync>`).
///
/// # Input Parameters
/// * MaxEntries ([`prim@usize`]) (optional) - The maximum number of entries that can be in this
/// hashtable at any given time. The default value is 1_000.
/// * MaxLoadFactor ([`prim@usize`]) (optional) - The maximum load factor of the hashtable. The hashtable
/// is an array based hashtable and it has a fixed size. Once the load factor is reached, insertions will
/// return an error. The hashtable uses linear probing to handle collisions. The max_load_factor makes sure
/// no additional insertions occur at a given ratio of entries to capacity in the array. Note that
/// MaxEntries can always be inserted, it's the capacity of the array that becomes larger as this ratio
/// goes down. So if 100 MaxEntries are specified and the MaxLoadFactor is 0.5, a 200 slot array
/// will be used and 100 entries will be allowed. The default MaxLoadFactor is 0.7.
/// * GlobalSlabAllocator ([`bool`]) (optional) - If true, the [`crate::global_slab_allocator`] is
/// used instead of using an internally built slab allocator. The global slab allocator is
/// thread_local and the returned value cannot be passed to other threads. The default value is
/// true.
/// * SlabSize ([`prim@usize`]) (optional) - The size of slabs for the [`crate::SlabAllocator`] associated
/// with this [`crate::Hashtable`]. This option is only allowed if GlobalSlabAllocator is false.
/// * SlabCount ([`prim@usize`]) (optional) - The count of slabs. This option is only allowed if
/// GlobalSlabAllocator is false.
///
/// # Returns
///
/// A Ok(`Box<dyn Hashtable<K, V> + Send + Sync>`) on success or a [`bmw_err::Error`] on failure.
///
/// # Errors
///
/// * [`bmw_err::ErrKind::Configuration`] - If any values are specified other than the allowed
/// values mentioned above or if there are any duplicate parameters specified.
/// * [`bmw_err::ErrKind::Configuration`] - If SlabSize or SlabCount are not specified.
/// * [`bmw_err::ErrKind::Configuration`] - If GlobalSlabAllocator is true (not allowed for sync)
/// * [`bmw_err::ErrKind::IllegalArgument`] - If the parameters specified for the SlabSize or
/// SlabCount are not valid. See [`crate::SlabAllocator`].
///
/// # Examples
///```      
/// use bmw_util::*;
/// use bmw_log::*;
/// use bmw_err::*;
///
/// fn main() -> Result<(), Error> {
///         // create a hashtable with the specified parameters
///         let mut hashtable = hashtable_sync_box!(
///                 MaxEntries(1_000),
///                 MaxLoadFactor(0.9),
///                 GlobalSlabAllocator(false),
///                 SlabSize(100),
///                 SlabCount(100)
///         )?;
///
///         // do an insert, rust will figure out what type is being inserted
///         hashtable.insert(&1u64, &10u8)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashtable.get(&1u64)?, Some(10u8));
///
///         // a hashtable_sync_box cannot be created with the global slab allocator
///         // at a minimum GlobalSlabAllocator must be false and SlabSize/SlabCount must
///         // be specified
///         let mut hashtable = hashtable_sync_box!(
///             GlobalSlabAllocator(false),
///             SlabSize(100),
///             SlabCount(200),
///         )?;
///
///         hashtable.insert(&1i32, &10i128)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashtable.get(&1i32)?, Some(10i128));
///
///         // assert that this entry is not in the table
///         assert_eq!(hashtable.get(&2i32)?, None);
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! hashtable_sync_box {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashtable_sync_box(v)
        }};
}

/// The [`crate::hashset`] macro builds a [`crate::Hashset`] with the specified configuration.
///
/// # Input Parameters
/// * MaxEntries ([`prim@usize`]) (optional) - The maximum number of entries that can be in this hashset
/// at any given time. The default value is 1_000.
/// * MaxLoadFactor ([`prim@usize`]) (optional) - The maximum load factor of the hashset. The hashset is an
/// array based hashset and it has a fixed size. Once the load factor is reached, insertions will return an
/// error. The hashset uses linear probing to handle collisions. The max_load_factor makes sure no
/// additional insertions occur at a given ratio of entries to capacity in the array. Note that
/// MaxEntries can always be inserted, it's the capacity of the array that becomes larger as this ratio
/// goes down. So if 100 MaxEntries are specified and the MaxLoadFactor is 0.5, a 200 slot array
/// will be used and 100 entries will be allowed. The default MaxLoadFactor is 0.7.
/// * GlobalSlabAllocator ([`bool`]) (optional) - If true, the [`crate::global_slab_allocator`] is
/// used instead of using an internally built slab allocator. The global slab allocator is
/// thread_local and the returned value cannot be passed to other threads. The default value is
/// true.
/// * SlabSize ([`prim@usize`]) (optional) - The size of slabs for the [`crate::SlabAllocator`] associated
/// with this [`crate::Hashset`]. This option is only allowed if GlobalSlabAllocator is false.
/// * SlabCount ([`prim@usize`]) (optional) - The count of slabs. This option is only allowed if
/// GlobalSlabAllocator is false.
///
/// # Returns
///
/// A Ok(`impl Hashset<K>`) on success or a [`bmw_err::Error`] on failure.
///
/// # Errors
///
/// * [`bmw_err::ErrKind::Configuration`] - If any values are specified other than the allowed
/// values mentioned above or if there are any duplicate parameters specified.
/// * [`bmw_err::ErrKind::Configuration`] - If only one of SlabSize and SlabCount are specified.
/// * [`bmw_err::ErrKind::Configuration`] - If GlobalSlabAllocator is true and SlabSize or
/// SlabCount are specified.
/// * [`bmw_err::ErrKind::Configuration`] - If GlobalSlabAllocator is false and SlabSize or
/// SlabCount are not specified.
/// * [`bmw_err::ErrKind::IllegalArgument`] - If the parameters specified for the SlabSize or
/// SlabCount are not valid. See [`crate::SlabAllocator`].
///
/// # Examples
///```
/// use bmw_util::*;
/// use bmw_log::*;
/// use bmw_err::*;
///
/// fn main() -> Result<(), Error> {
///         // create a hashset with the specified parameters
///         let mut hashset = hashset!(
///                 MaxEntries(1_000),
///                 MaxLoadFactor(0.9),
///                 GlobalSlabAllocator(false),
///                 SlabSize(100),
///                 SlabCount(100)
///         )?;
///
///         // do an insert, rust will figure out what type is being inserted
///         hashset.insert(&1)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashset.contains(&1)?, true);
///
///         // create another hashset with defaults, this time the global slab allocator will be
///         // used. Since we did not initialize it default values will be used.
///         let mut hashset = hashset!()?;
///
///         // do an insert, rust will figure out what type is being inserted
///         hashset.insert(&1)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashset.contains(&1)?, true);
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! hashset {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashset(v)
        }};
}

/// The [`crate::hashset_box`] macro is the `boxed` version of the [`crate::hashset`] macro. It returns a
/// [`crate::Hashset`] within a [`std::boxed::Box`] (`Box<dyn HashSet<K>>`) with the specified configuration.
///
/// # Input Parameters
/// * MaxEntries ([`prim@usize`]) (optional) - The maximum number of entries that can be in this hashset
/// at any given time. The default value is 1_000.
/// * MaxLoadFactor ([`prim@usize`]) (optional) - The maximum load factor of the hashset. The hashset is an
/// array based hashset and it has a fixed size. Once the load factor is reached, insertions will return an
/// error. The hashset uses linear probing to handle collisions. The max_load_factor makes sure no
/// additional insertions occur at a given ratio of entries to capacity in the array. Note that
/// MaxEntries can always be inserted, it's the capacity of the array that becomes larger as this ratio
/// goes down. So if 100 MaxEntries are specified and the MaxLoadFactor is 0.5, a 200 slot array
/// will be used and 100 entries will be allowed. The default MaxLoadFactor is 0.7.
/// * GlobalSlabAllocator ([`bool`]) (optional) - If true, the [`crate::global_slab_allocator`] is
/// used instead of using an internally built slab allocator. The global slab allocator is
/// thread_local and the returned value cannot be passed to other threads. The default value is
/// true.
/// * SlabSize ([`prim@usize`]) (optional) - The size of slabs for the [`crate::SlabAllocator`] associated
/// with this [`crate::Hashset`]. This option is only allowed if GlobalSlabAllocator is false.
/// * SlabCount ([`prim@usize`]) (optional) - The count of slabs. This option is only allowed if
/// GlobalSlabAllocator is false.
///
/// # Returns
///
/// A Ok(`Box<dyn Hashset<K>>`) on success or a [`bmw_err::Error`] on failure.
///
/// # Errors
///
/// * [`bmw_err::ErrKind::Configuration`] - If any values are specified other than the allowed
/// values mentioned above or if there are any duplicate parameters specified.
/// * [`bmw_err::ErrKind::Configuration`] - If only one of SlabSize and SlabCount are specified.
/// * [`bmw_err::ErrKind::Configuration`] - If GlobalSlabAllocator is true and SlabSize or
/// SlabCount are specified.
/// * [`bmw_err::ErrKind::Configuration`] - If GlobalSlabAllocator is false and SlabSize or
/// SlabCount are not specified.
/// * [`bmw_err::ErrKind::IllegalArgument`] - If the parameters specified for the SlabSize or
/// SlabCount are not valid. See [`crate::SlabAllocator`].
///
/// # Examples
///```
/// use bmw_util::*;
/// use bmw_log::*;
/// use bmw_err::*;
///
/// fn main() -> Result<(), Error> {
///         // create a hashset with the specified parameters
///         let mut hashset = hashset_box!(
///                 MaxEntries(1_000),
///                 MaxLoadFactor(0.9),
///                 GlobalSlabAllocator(false),
///                 SlabSize(100),
///                 SlabCount(100)
///         )?;
///
///         // do an insert, rust will figure out what type is being inserted
///         hashset.insert(&1u64)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashset.contains(&1u64)?, true);
///
///         // create a hashset with the global slab allocator
///         let mut hashset = hashset_box!()?;
///
///         hashset.insert(&1u8)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashset.contains(&1u8)?, true);
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! hashset_box {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashset_box(v)
        }};
}

/// The [`crate::hashset_sync`] macro is the `sync` version of the [`crate::hashset`] macro.
/// It returns an impl of a [`crate::Hashset`] + Send + Sync (`impl HashSet<K> + Send + Sync`)
/// with the specified configuration.
///
/// # Input Parameters
/// * MaxEntries ([`prim@usize`]) (optional) - The maximum number of entries that can be in this hashset
/// at any given time. The default value is 1_000.
/// * MaxLoadFactor ([`prim@usize`]) (optional) - The maximum load factor of the hashset. The hashset is an
/// array based hashset and it has a fixed size. Once the load factor is reached, insertions will return an
/// error. The hashset uses linear probing to handle collisions. The max_load_factor makes sure no
/// additional insertions occur at a given ratio of entries to capacity in the array. Note that
/// MaxEntries can always be inserted, it's the capacity of the array that becomes larger as this ratio
/// goes down. So if 100 MaxEntries are specified and the MaxLoadFactor is 0.5, a 200 slot array
/// will be used and 100 entries will be allowed. The default MaxLoadFactor is 0.7.
/// * GlobalSlabAllocator ([`bool`]) (optional) - If true, the [`crate::global_slab_allocator`] is
/// used instead of using an internally built slab allocator. The global slab allocator is
/// thread_local and the returned value cannot be passed to other threads. The default value is
/// true.
/// * SlabSize ([`prim@usize`]) (optional) - The size of slabs for the [`crate::SlabAllocator`] associated
/// with this [`crate::Hashset`]. This option is only allowed if GlobalSlabAllocator is false.
/// * SlabCount ([`prim@usize`]) (optional) - The count of slabs. This option is only allowed if
/// GlobalSlabAllocator is false.
///
/// # Returns
///
/// A Ok(`impl Hashset<K> + Send + Sync`) on success or a [`bmw_err::Error`] on failure.
///
/// # Errors
///
/// * [`bmw_err::ErrKind::Configuration`] - If any values are specified other than the allowed
/// values mentioned above or if there are any duplicate parameters specified.
/// * [`bmw_err::ErrKind::Configuration`] - If SlabSize or SlabCount are not specified.
/// * [`bmw_err::ErrKind::Configuration`] - If GlobalSlabAllocator is true (not allowed for IsSync)
/// * [`bmw_err::ErrKind::IllegalArgument`] - If the parameters specified for the SlabSize or
/// SlabCount are not valid. See [`crate::SlabAllocator`].
///
/// # Examples
///```
/// use bmw_util::*;
/// use bmw_log::*;
/// use bmw_err::*;
///
/// fn main() -> Result<(), Error> {
///         // create a hashset with the specified parameters
///         let mut hashset = hashset_sync!(
///                 MaxEntries(1_000),
///                 MaxLoadFactor(0.9),
///                 GlobalSlabAllocator(false),
///                 SlabSize(100),
///                 SlabCount(100)
///         )?;
///
///         // do an insert, rust will figure out what type is being inserted
///         hashset.insert(&1u64)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashset.contains(&1u64)?, true);
///
///         // a hashset_sync cannot be created with the global slab allocator
///         // at a minimum GlobalSlabAllocator must be false and SlabSize/SlabCount must
///         // be specified
///         let mut hashset = hashset_sync!(
///             GlobalSlabAllocator(false),
///             SlabSize(100),
///             SlabCount(200),
///         )?;
///
///         hashset.insert(&1u8)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashset.contains(&1u8)?, true);
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! hashset_sync {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashset_sync(v)
        }};
}

/// The [`crate::hashset_sync_box`] macro is the `sync` and `boxed` version of the [`crate::hashset`]
/// macro. It returns an [`std::boxed::Box`] of a [`crate::Hashset`] + Send + Sync
/// (`Box<dyn HashSet<K> + Send + Sync>`) with the specified configuration.
///
/// # Input Parameters
/// * MaxEntries ([`prim@usize`]) (optional) - The maximum number of entries that can be in this hashset
/// at any given time. The default value is 1_000.
/// * MaxLoadFactor ([`prim@usize`]) (optional) - The maximum load factor of the hashset. The hashset is an
/// array based hashset and it has a fixed size. Once the load factor is reached, insertions will return an
/// error. The hashset uses linear probing to handle collisions. The max_load_factor makes sure no
/// additional insertions occur at a given ratio of entries to capacity in the array. Note that
/// MaxEntries can always be inserted, it's the capacity of the array that becomes larger as this ratio
/// goes down. So if 100 MaxEntries are specified and the MaxLoadFactor is 0.5, a 200 slot array
/// will be used and 100 entries will be allowed. The default MaxLoadFactor is 0.7.
/// * GlobalSlabAllocator ([`bool`]) (optional) - If true, the [`crate::global_slab_allocator`] is
/// used instead of using an internally built slab allocator. The global slab allocator is
/// thread_local and the returned value cannot be passed to other threads. The default value is
/// true.
/// * SlabSize ([`prim@usize`]) (optional) - The size of slabs for the [`crate::SlabAllocator`] associated
/// with this [`crate::Hashset`]. This option is only allowed if GlobalSlabAllocator is false.
/// * SlabCount ([`prim@usize`]) (optional) - The count of slabs. This option is only allowed if
/// GlobalSlabAllocator is false.
///
/// # Returns
///
/// A Ok(`Box<dyn HashSet<K> + Send + Sync>`) on success or a [`bmw_err::Error`] on failure.
///
/// # Errors
///
/// * [`bmw_err::ErrKind::Configuration`] - If any values are specified other than the allowed
/// values mentioned above or if there are any duplicate parameters specified.
/// * [`bmw_err::ErrKind::Configuration`] - If SlabSize or SlabCount are not specified.
/// * [`bmw_err::ErrKind::Configuration`] - If GlobalSlabAllocator is true (not allowed for IsSync)
/// * [`bmw_err::ErrKind::IllegalArgument`] - If the parameters specified for the SlabSize or
/// SlabCount are not valid. See [`crate::SlabAllocator`].
///
/// # Examples
///```
/// use bmw_util::*;
/// use bmw_log::*;
/// use bmw_err::*;
///
/// fn main() -> Result<(), Error> {
///         // create a hashset with the specified parameters
///         let mut hashset = hashset_sync_box!(
///                 MaxEntries(1_000),
///                 MaxLoadFactor(0.9),
///                 GlobalSlabAllocator(false),
///                 SlabSize(100),
///                 SlabCount(100)
///         )?;
///
///         // do an insert, rust will figure out what type is being inserted
///         hashset.insert(&1u64)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashset.contains(&1u64)?, true);
///
///         // a hashset_sync_box cannot be created with the global slab allocator
///         // at a minimum GlobalSlabAllocator must be false and SlabSize/SlabCount must
///         // be specified
///         let mut hashset = hashset_sync_box!(
///             GlobalSlabAllocator(false),
///             SlabSize(100),
///             SlabCount(200),
///         )?;
///
///         hashset.insert(&1u8)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashset.contains(&1u8)?, true);
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! hashset_sync_box {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashset_sync_box(v)
        }};
}

/// The [`crate::list`] macro is used to create implementations of [`crate::List`]. This macro uses the global slab
/// allocator. To use an internal slab allocator, see [`crate::UtilBuilder::build_list`]. This macro has the same
/// syntax as the [`std::vec!`] macro. Note that this macro and the builder function both
/// return an implementation of the [`crate::SortableList`] trait that uses a linked-list like
/// implementation.
///
/// # Examples
///```
/// // create a list and iterate through it
/// use bmw_util::*;
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///     let list = list![1, 2, 3, 4];
///
///     info!("list={:?}", list)?;
///
///     let mut i = 1;
///
///     for x in list.iter() {
///         assert_eq!(x, i);
///         i += 1;
///     }
///
///     assert!(list_eq!(list, list![1, 2, 3, 4]));
///
///     Ok(())
/// }
///```
///
///```
/// // sort a list
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///     // create two lists
///     let mut list1 = list![];
///     let mut list2 = list![];
///
///     // add 0..10 to the list
///     for i in 0..10 {
///         list1.push(i)?;
///     }
///
///     // add 10..0 to the list
///     for i in (0..10).rev() {
///         list2.push(i)?;
///     }
///
///     // sort the lists using the unstable and stable sort functions (underlying rust fns used)
///     list1.sort_unstable()?;
///     list2.sort()?;
///
///     // ensure they are equal after the sorting takes place
///     assert!(list_eq!(list1, list2));
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! list {
    ( $( $x:expr ),* ) => {
        {
            use bmw_util::List;
            #[allow(unused_mut)]
            let mut temp_list = bmw_util::UtilBuilder::build_list(vec![])?;
            $(
                temp_list.push($x)?;
            )*
            temp_list
        }
    };
}

/// The [`crate::list_box`] macro is the `boxed` version of [`crate::list`]. This macro uses the global slab
/// allocator. To use an internal slab allocator, see [`crate::UtilBuilder::build_list_box`]. This macro has the same
/// syntax as the [`std::vec!`] macro. Note that this macro and the builder function both
/// return an implementation of the [`crate::SortableList`] trait in the form of `Box<dyn
/// SortableList<K>>` that uses a linked-list like implementation.
///
/// # Examples
///```
/// // create a list and iterate through it
/// use bmw_util::*;
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///     let list = list_box![1, 2, 3, 4];
///
///     info!("list={:?}", list)?;
///
///     let mut i = 1;
///
///     for x in list.iter() {
///         assert_eq!(x, i);
///         i += 1;
///     }       
///             
///     assert!(list_eq!(list, list_box![1, 2, 3, 4]));
///             
///     Ok(())  
/// }   
///```
///
///```
/// // sort a list
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///     // create two lists
///     let mut list1 = list_box![];
///     let mut list2 = list_box![];
///
///     // add 0..10 to the list
///     for i in 0..10 {
///         list1.push(i)?;
///     }
///
///     // add 10..0 to the list
///     for i in (0..10).rev() {
///         list2.push(i)?;
///     }
///
///     // sort the lists using the unstable and stable sort functions (underlying rust fns used)
///     list1.sort_unstable()?;
///     list2.sort()?;
///
///     // ensure they are equal after the sorting takes place
///     assert!(list_eq!(list1, list2));
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! list_box {
    ( $( $x:expr ),* ) => {
        {
            #[allow(unused_mut)]
            let mut temp_list = bmw_util::UtilBuilder::build_list_box(vec![])?;
            $(
                temp_list.push($x)?;
            )*
            temp_list
        }
    };
}

/// The [`crate::list_sync`] macro is the `sync` version of [`crate::list`]. Since this macro
/// cannot use the global_slab_allocator, it must use a default slab allocator coniguration. It
/// uses a SlabSize of 256 and with a SlabCount of 10_000. If different parameters are desired, the
/// [`crate::UtilBuilder::build_list_sync`] function should be used.
///
/// # Examples
///```
/// // create a list and iterate through it
/// use bmw_util::*;
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///     let list = list_sync![1,2,3,4];
///
///     info!("list={:?}", list)?;
///
///     let mut i = 1;
///
///     for x in list.iter() {
///         assert_eq!(x, i);
///         i += 1;
///     }
///
///     assert!(list_eq!(list, list_sync![1, 2, 3, 4]));
///
///     Ok(())
/// }
///```
///
///```
/// // sort a list
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///     // create two lists
///     let mut list1 = list_sync![];
///     let mut list2 = list_sync![];
///
///     // add 0..10 to the list
///     for i in 0..10 {
///         list1.push(i)?;
///     }
///
///     // add 10..0 to the list
///     for i in (0..10).rev() {
///         list2.push(i)?;
///     }
///
///     // sort the lists using the unstable and stable sort functions (underlying rust fns used)
///     list1.sort_unstable()?;
///     list2.sort()?;
///
///     // ensure they are equal after the sorting takes place
///     assert!(list_eq!(list1, list2));
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! list_sync {
    ( $( $x:expr ),* ) => {
        {
            #[allow(unused_mut)]
            let mut temp_list = bmw_util::UtilBuilder::build_list_sync(vec![
                bmw_conf::ConfigOption::GlobalSlabAllocator(false),
                bmw_conf::ConfigOption::SlabSize(256),
                bmw_conf::ConfigOption::SlabCount(10_000),
            ])?;
            $(
                temp_list.push($x)?;
            )*
            temp_list
        }
    };
}

/// The [`crate::list_sync_box`] macro is the `sync` and `boxed` version of [`crate::list`]. Since this macro
/// cannot use the global_slab_allocator, it must use a default slab allocator coniguration. It
/// uses a SlabSize of 256 and with a SlabCount of 10_000. If different parameters are desired, the
/// [`crate::UtilBuilder::build_list_sync_box`] function should be used.
///
/// # Examples
///```
/// // create a list and iterate through it
/// use bmw_util::*;
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///     let list = list_sync_box![1,2,3,4];
///
///     info!("list={:?}", list)?;
///
///     let mut i = 1;
///
///     for x in list.iter() {
///         assert_eq!(x, i);
///         i += 1;
///     }
///
///     assert!(list_eq!(list, list_sync_box![1, 2, 3, 4]));
///
///     Ok(())
/// }
///```
///
///```
/// // sort a list
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///     // create two lists
///     let mut list1 = list_sync_box![];
///     let mut list2 = list_sync_box![];
///
///     // add 0..10 to the list
///     for i in 0..10 {
///         list1.push(i)?;
///     }
///
///     // add 10..0 to the list
///     for i in (0..10).rev() {
///         list2.push(i)?;
///     }
///
///     // sort the lists using the unstable and stable sort functions (underlying rust fns used)
///     list1.sort_unstable()?;
///     list2.sort()?;
///
///     // ensure they are equal after the sorting takes place
///     assert!(list_eq!(list1, list2));
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! list_sync_box {
    ( $( $x:expr ),* ) => {
        {
            #[allow(unused_mut)]
            let mut temp_list = bmw_util::UtilBuilder::build_list_sync_box(vec![
                bmw_conf::ConfigOption::GlobalSlabAllocator(false),
                bmw_conf::ConfigOption::SlabSize(256),
                bmw_conf::ConfigOption::SlabCount(10_000),
            ])?;
            $(
                temp_list.push($x)?;
            )*
            temp_list
        }
    };
}

/// The [`crate::array!`] macro builds an [`crate::Array`].
///
/// # Input Paramters
/// * size ([`prim@usize`]) (required) - the size of the array
/// * default ([`bmw_ser::Serializable`]) (required) - a reference to the value to initialize the array with
///
/// # Return
/// Returns [`crate::Array`] on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///         let arr = array!(10, &0)?;
///
///         for x in arr.iter() {
///                 assert_eq!(x, &0);
///         }
///
///         assert_eq!(arr[3], 0);
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! array {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_array($size, $default)
	}};
}

/// The [`crate::array_list`] macro builds an [`crate::ArrayList`] in the form of an impl
/// SortableList.
///
/// # Input Parameters
/// * size ([`prim@usize`]) (required) - the size of the array
/// * default ([`bmw_ser::Serializable`]) (required) - a reference to the value to initialize the array with
///
/// # Return
/// Returns [`crate::SortableList`] on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// // create an array_list and iterate through it
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///         let mut arr = array_list!(10, &0)?;
///         for _ in 0..10 {
///                 arr.push(0)?;
///         }
///
///         info!("arr = {:?}", arr)?;
///
///         for x in arr.iter() {
///                 assert_eq!(x, 0);
///         }
///
///         Ok(())
/// }
///
///```
///
///```
/// // sort an array list
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///     // create two array lists
///     let mut arr1 = array_list!(10, &0)?;
///     let mut arr2 = array_list!(10, &0)?;
///
///     // add 0..10 to the list
///     for i in 0..10 {
///         arr1.push(i)?;
///     }
///
///     assert!(arr1.push(0).is_err()); // it's full
///
///     // add 10..0 to the list
///     for i in (0..10).rev() {
///         arr2.push(i)?;
///     }
///
///     assert!(arr2.push(0).is_err()); // it's full
///
///     // sort the arrays using the unstable and stable sort functions (underlying rust fns used)
///     arr1.sort_unstable()?;
///     arr2.sort()?;
///
///     // ensure they are equal after the sorting takes place
///     assert!(list_eq!(arr1, arr2));
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! array_list {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_array_list($size, $default)
	}};
}

/// The `boxed` form of [`crate::array_list`]. This macro builds an [`crate::ArrayList`] in the form of a
/// `Box<dyn SortableList<T>>`.
///
/// # Input Parameters
/// * size ([`prim@usize`]) (required) - the size of the array
/// * default ([`bmw_ser::Serializable`]) (required) - a reference to the value to initialize the array with
///
/// # Return
/// Returns [`crate::SortableList`] on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// // create an array_list and iterate through it
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///         let mut arr = array_list_box!(10, &0)?;
///         for _ in 0..10 {
///                 arr.push(0)?;
///         }
///
///         info!("arr = {:?}", arr)?;
///
///         for x in arr.iter() {
///                 assert_eq!(x, 0);
///         }
///
///         Ok(())
/// }
///
///```
///
///```
/// // sort an array list
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///     // create two array lists
///     let mut arr1 = array_list_box!(10, &0)?;
///     let mut arr2 = array_list_box!(10, &0)?;
///
///     // add 0..10 to the list
///     for i in 0..10 {
///         arr1.push(i)?;
///     }
///
///     assert!(arr1.push(0).is_err()); // it's full
///
///     // add 10..0 to the list
///     for i in (0..10).rev() {
///         arr2.push(i)?;
///     }
///
///     assert!(arr2.push(0).is_err()); // it's full
///
///     // sort the arrays using the unstable and stable sort functions (underlying rust fns used)
///     arr1.sort_unstable()?;
///     arr2.sort()?;
///
///     // ensure they are equal after the sorting takes place
///     assert!(list_eq!(arr1, arr2));
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! array_list_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_array_list_box($size, $default)
	}};
}

/// The `sync` form of [`crate::array_list`]. This macro builds an [`crate::ArrayList`] in the form of a
/// `<impl SortableList<T> + Send + Sync`
///
/// # Input Parameters
/// * size ([`prim@usize`]) (required) - the size of the array
/// * default ([`bmw_ser::Serializable`]) (required) - a reference to the value to initialize the array with
///
/// # Return
/// Returns [`crate::SortableList`] on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// // create an array_list and iterate through it
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///         let mut arr = array_list_sync!(10, &0)?;
///         for _ in 0..10 {
///                 arr.push(0)?;
///         }
///
///         info!("arr = {:?}", arr)?;
///
///         for x in arr.iter() {
///                 assert_eq!(x, 0);
///         }
///
///         Ok(())
/// }
///
///```
///
///```
/// // sort an array list
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///     // create two array lists
///     let mut arr1 = array_list_sync!(10, &0)?;
///     let mut arr2 = array_list_sync!(10, &0)?;
///
///     // add 0..10 to the list
///     for i in 0..10 {
///         arr1.push(i)?;
///     }
///
///     assert!(arr1.push(0).is_err()); // it's full
///
///     // add 10..0 to the list
///     for i in (0..10).rev() {
///         arr2.push(i)?;
///     }
///
///     assert!(arr2.push(0).is_err()); // it's full
///
///     // sort the arrays using the unstable and stable sort functions (underlying rust fns used)
///     arr1.sort_unstable()?;
///     arr2.sort()?;
///
///     // ensure they are equal after the sorting takes place
///     assert!(list_eq!(arr1, arr2));
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! array_list_sync {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_array_list_sync($size, $default)
	}};
}

/// The `sync_boxed` form of [`crate::array_list`]. This macro builds an [`crate::ArrayList`] in the form of a
/// `Box<dyn SortableList<T> + Send + Sync>`.
///
/// # Input Parameters
/// * size ([`prim@usize`]) (required) - the size of the array
/// * default ([`bmw_ser::Serializable`]) (required) - a reference to the value to initialize the array with
///
/// # Return
/// Returns [`crate::SortableList`] on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// // create an array_list and iterate through it
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///         let mut arr = array_list_sync_box!(10, &0)?;
///         for _ in 0..10 {
///                 arr.push(0)?;
///         }
///
///         info!("arr = {:?}", arr)?;
///
///         for x in arr.iter() {
///                 assert_eq!(x, 0);
///         }
///
///         Ok(())
/// }
///
///```
///
///```
/// // sort an array list
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///     // create two array lists
///     let mut arr1 = array_list_sync_box!(10, &0)?;
///     let mut arr2 = array_list_sync_box!(10, &0)?;
///
///     // add 0..10 to the list
///     for i in 0..10 {
///         arr1.push(i)?;
///     }
///
///     assert!(arr1.push(0).is_err()); // it's full
///
///     // add 10..0 to the list
///     for i in (0..10).rev() {
///         arr2.push(i)?;
///     }
///
///     assert!(arr2.push(0).is_err()); // it's full
///
///     // sort the arrays using the unstable and stable sort functions (underlying rust fns used)
///     arr1.sort_unstable()?;
///     arr2.sort()?;
///
///     // ensure they are equal after the sorting takes place
///     assert!(list_eq!(arr1, arr2));
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! array_list_sync_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_array_list_sync_box($size, $default)
	}};
}

/// The [`crate::queue`] macro creates a [`crate::Queue`] implementation with the specified
/// parameters. Note that like [`crate::Stack`], this is a bounded queue. The queue is returned
/// as an `impl Queue<T>`.
///
/// # Input Parameters
/// * size ([`prim@usize`]) (required) - the size of the underlying array
/// * default ([`bmw_ser::Serializable`]) (required) - a reference to the value to initialize the array with
/// for the queue, these values are never used, but a default is needed to initialize the
/// underlying array.
/// # Return
/// Returns `Ok(impl Queue<T>)` on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///         let mut queue = queue!(10, &0)?;
///
///         for i in 0..10 {
///                 queue.enqueue(i)?;
///         }
///
///         for i in 0..10 {
///                 let v = queue.dequeue().unwrap();
///                 assert_eq!(v, &i);
///         }
///         
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! queue {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_queue($size, $default)
	}};
}

/// The [`crate::queue_box`] macro is the `boxed` version of [`crate::queue`]. This macro creates a [`crate::Queue`]
/// with the specified parameters. The queue is returned as a `Box<dyn Queue<T>>`.
///
/// # Input Parameters
/// * size ([`prim@usize`]) (required) - the size of the underlying array
/// * default ([`bmw_ser::Serializable`]) (required) - a reference to the value to initialize the array with
/// for the queue, these values are never used, but a default is needed to initialize the
/// underlying array.
/// # Return
/// Returns `Ok(Box<dyn Queue<T>>)` on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///         let mut queue = queue_box!(10, &0)?;
///
///         for i in 0..10 {
///                 queue.enqueue(i)?;
///         }
///
///         for i in 0..10 {
///                 let v = queue.dequeue().unwrap();
///                 assert_eq!(v, &i);
///         }
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! queue_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_queue_box($size, $default)
	}};
}

/// The [`crate::queue_sync`] macro is the `sync` version of [`crate::queue`]. This macro creates a [`crate::Queue`]
/// with the specified parameters. The queue is returned as a `impl Queue<T> + Send + Sync`.
///
/// # Input Parameters
/// * size ([`prim@usize`]) (required) - the size of the underlying array
/// * default ([`bmw_ser::Serializable`]) (required) - a reference to the value to initialize the array with
/// for the queue, these values are never used, but a default is needed to initialize the
/// underlying array.
/// # Return
/// Returns `Ok(impl Queue<T> + Send + Sync)` on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///         let mut queue = queue_sync!(10, &0)?;
///
///         for i in 0..10 {
///                 queue.enqueue(i)?;
///         }
///
///         for i in 0..10 {
///                 let v = queue.dequeue().unwrap();
///                 assert_eq!(v, &i);
///         }
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! queue_sync {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_queue_sync($size, $default)
	}};
}

/// The [`crate::queue_sync_box`] macro is the `sync` and `boxed` version of [`crate::queue`].
/// This macro creates a [`crate::Queue`] with the specified parameters. The queue is returned
/// as a `Box<dyn Queue<T> + Send + Sync>`.
///
/// # Input Parameters
/// * size ([`prim@usize`]) (required) - the size of the underlying array
/// * default ([`bmw_ser::Serializable`]) (required) - a reference to the value to initialize the array with
/// for the queue, these values are never used, but a default is needed to initialize the
/// underlying array.
/// # Return
/// Returns `Ok(Box<dyn Queue<T> + Send + Sync>)` on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///         let mut queue = queue_sync_box!(10, &0)?;
///
///         for i in 0..10 {
///                 queue.enqueue(i)?;
///         }
///
///         for i in 0..10 {
///                 let v = queue.dequeue().unwrap();
///                 assert_eq!(v, &i);
///         }
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! queue_sync_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_queue_sync_box($size, $default)
	}};
}

/// The [`crate::stack`] macro creates a [`crate::Stack`] implementation with the specified
/// parameters. Note that like [`crate::Queue`], this is a bounded stack. The stack is returned
/// as an `impl Stack<T>`.
///
/// # Input Parameters
/// * size ([`prim@usize`]) (required) - the size of the underlying array
/// * default (required) - a reference to the value to initialize the array with
/// for the stack, these values are never used, but a default is needed to initialize the
/// underlying array.
/// # Return
/// Returns `Ok(impl Stack<T>)` on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///         let mut stack = stack!(10, &0)?;
///
///         for i in 0..10 {
///                 stack.push(i)?;
///         }
///
///         for i in (0..10).rev() {
///                 let v = stack.pop().unwrap();
///                 assert_eq!(v, &i);
///         }
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! stack {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_stack($size, $default)
	}};
}

/// The [`crate::stack_box`] macro is the `boxed` version of [`crate::stack`] . This macro creates
/// a [`crate::Stack`] implementation with the specified parameters. Note that like [`crate::Queue`],
/// this is a bounded stack. The stack is returned as a `Box<dyn Stack<T>>`.
///
/// # Input Parameters
/// * size ([`prim@usize`]) (required) - the size of the underlying array
/// * default (required) - a reference to the value to initialize the array with
/// for the stack, these values are never used, but a default is needed to initialize the
/// underlying array.
/// # Return
/// Returns `Ok(Box<dyn Stack<T>>)` on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///         let mut stack = stack_box!(10, &0)?;
///
///         for i in 0..10 {
///                 stack.push(i)?;
///         }
///
///         for i in (0..10).rev() {
///                 let v = stack.pop().unwrap();
///                 assert_eq!(v, &i);
///         }
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! stack_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_stack_box($size, $default)
	}};
}

/// The [`crate::stack_sync`] macro is the `sync` version of [`crate::stack`] . This macro creates
/// a [`crate::Stack`] implementation with the specified parameters. Note that like [`crate::Queue`],
/// this is a bounded stack. The stack is returned as a `impl Stack<T> + Send + Sync`.
///
/// # Input Parameters
/// * size ([`prim@usize`]) (required) - the size of the underlying array
/// * default (required) - a reference to the value to initialize the array with
/// for the stack, these values are never used, but a default is needed to initialize the
/// underlying array.
/// # Return
/// Returns `Ok(impl Stack<T>)` on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///         let mut stack = stack_sync!(10, &0)?;
///
///         for i in 0..10 {
///                 stack.push(i)?;
///         }
///
///         for i in (0..10).rev() {
///                 let v = stack.pop().unwrap();
///                 assert_eq!(v, &i);
///         }
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! stack_sync {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_stack_sync($size, $default)
	}};
}

/// The [`crate::stack_sync_box`] macro is the `sync` and `boxed` version of [`crate::stack`] . This macro creates
/// a [`crate::Stack`] implementation with the specified parameters. Note that like [`crate::Queue`],
/// this is a bounded stack. The stack is returned as a `Box<dyn Stack<T> + Send + Sync>`.
///
/// # Input Parameters
/// * size ([`prim@usize`]) (required) - the size of the underlying array
/// * default (required) - a reference to the value to initialize the array with
/// for the stack, these values are never used, but a default is needed to initialize the
/// underlying array.
/// # Return
/// Returns `Ok(Box<dyn Stack<T> + Send + Sync>)` on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///         let mut stack = stack_sync_box!(10, &0)?;
///
///         for i in 0..10 {
///                 stack.push(i)?;
///         }
///
///         for i in (0..10).rev() {
///                 let v = stack.pop().unwrap();
///                 assert_eq!(v, &i);
///         }
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! stack_sync_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_stack_sync_box($size, $default)
	}};
}

/// Append list2 to list1.
#[macro_export]
macro_rules! list_append {
	($list1:expr, $list2:expr) => {{
		for x in $list2.iter() {
			$list1.push(x)?;
		}
	}};
}

/// Compares equality of list1 and list2.
#[macro_export]
macro_rules! list_eq {
	($list1:expr, $list2:expr) => {{
		let list1 = &$list1;
		let list2 = &$list2;
		let list1_size = list1.size();
		if list1_size != list2.size() {
			false
		} else {
			let mut ret = true;
			{
				let mut itt1 = list1.iter();
				let mut itt2 = list2.iter();
				for _ in 0..list1_size {
					if itt1.next() != itt2.next() {
						ret = false;
					}
				}
			}
			ret
		}
	}};
}

/// Macro used to configure/build a thread pool. Threadpools can be used to execute tasks in a set
/// of threads that is configurable.
///
/// # Input Parameters
/// * MaxSize([`prim@usize`]) (optional) - the maximum size that the thread pool, in terms of number of threads, may grow to.
/// The default value is MinSize.
/// * MinSize([`prim@usize`]) (optional) - the minumum size, in threads, that the thread pool will maintain. The
/// thread pool will add threads up until MaxSize, but never go below MinSize threads. The default
/// value is 1.
/// * SyncChannelSize([`prim@usize`]) (optional) - the size of the internal sync_channel used to send tasks to the
/// thread pool threads for execution. The default is 10.
///
/// # Return value
///
/// Upon success, this macro will return a [`crate::ThreadPoolHandle`].
///
/// # Errors
///
/// [`bmw_err::ErrKind::Configuration`] - If the configuration contained parameters other than
/// MaxSize, MinSize, or SyncChannelSize.
///
/// [`bmw_err::ErrKind::Configuration`] - If the configuration contained duplicate parameters.
///
/// [`bmw_err::ErrKind::Configuration`] - If the configuration of MinSize is 0 or if MaxSize is
/// less than MinSize.
///
/// # Examples
///
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_test::*;
/// use bmw_util::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///     info!("testing thread_pool macro")?;
///
///     // if only MinSize is specified, the thread pool maintains
///     // MinSize threads (in this case 4) at all times.
///     let mut tp = thread_pool!(MinSize(4))?;
///     tp.set_on_panic(move |id, e| -> Result<(), Error> {
///         info!("PANIC: id={},e={:?}", id, e)?;
///         Ok(())
///     })?;
///
///     tp.start()?;
///
///     execute!(tp, {
///          info!("executing a thread")?;
///          Ok(())
///     })?;
///
///     Ok(())
/// }
///```
/// Here's an example with other values configured. This also demonstrates how to block on a task
/// in another thread.
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_test::*;
/// use bmw_util::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///     info!("testing thread_pool macro")?;
///
///     let mut tp = thread_pool!(MinSize(4), MaxSize(5), SyncChannelSize(15))?;
///     tp.set_on_panic(move |id, e| -> Result<(), Error> {
///         info!("PANIC: id={},e={:?}", id, e)?;
///         Ok(())
///     })?;
///     
///     tp.start()?;
///     
///     let mut tph = execute!(tp, {
///          info!("executing a thread")?;
///          Ok(1)
///     })?;
///
///     let result = tph.block_on();
///
///     info!("Thread {} resulted in a value of {:?}", tph.id(), result)?;
///
///     assert_eq!(result, PoolResult::Ok(1));
///         
///     Ok(())
/// }
///```
/// Here's an example handling a thread panic.
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_test::*;
/// use bmw_util::*;
/// use bmw_deps::rand::random;
/// use std::sync::mpsc::sync_channel;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///     info!("testing thread_pool macro with panic")?;
///     let mut id_box = lock_box!(random())?;
///     let id_box_clone = id_box.clone();
///     let (tx, rx) = sync_channel(1);
///
///     let mut tp = thread_pool!(MinSize(4), MaxSize(5), SyncChannelSize(15))?;
///     tp.set_on_panic(move |id, e| -> Result<(), Error> {
///         info!("PANIC: id={},e={:?}", id, e)?;
///
///         wlock!(id_box) = id;
///         tx.send(());
///
///         Ok(())
///     })?;
///  
///     tp.start()?;
///
///     let mut tph = execute!(tp, {
///          info!("executing a thread")?;
///          if true {
///                 panic!("12345");
///          }
///          Ok(1)
///     })?;
///     
///     let result = tph.block_on();
///     
///     info!("Thread {} resulted in a value of {:?}", tph.id(), result)?;
///     
///     assert_eq!(
///         result,
///         PoolResult::Err(
///             err!(
///                 ErrKind::ThreadPanic,
///                 "thread pool panic: receiving on a closed channel"
///             )
///         )
///     );
///     rx.recv()?;
///     assert_eq!(rlock!(id_box_clone), tph.id());
///         
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! thread_pool {
        ( $( $match_items:tt)* ) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($match_items)*];
                bmw_util::UtilBuilder::build_thread_pool(v)
        }};
}

/// This macro executes a task in the specified [`crate::ThreadPool`].
///
/// # Input Parameters
/// * thread_pool (required) - the [`crate::ThreadPool`] to execute this task in.
/// * id (optional) - id to assign to this task. If not specified, a random id is assigned.
/// * task (required) - the task to execute in the thread pool.
///
/// # Return
/// The [`crate::ThreadPoolHandle`] associated with this task.
///
/// # Errors
/// * [`bmw_err::ErrKind::IllegalState`] - If the thread pool has not been started.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///         let mut tp = thread_pool!()?;
///
///         tp.set_on_panic(move |id, e| -> Result<(), Error> {
///             let e = e.downcast_ref::<&str>().unwrap_or(&"unknown panic type");
///             info!("PANIC: id={},e={}", id, e)?;
///             Ok(())
///         })?;
///
///         tp.start()?;
///
///         let tph = execute!(tp, {
///             info!("executing a task in another thread!")?;
///             Ok(101)
///         })?;
///
///         let res = block_on!(tph);
///         assert_eq!(res, PoolResult::Ok(101));
///
///         Ok(())
/// }
///
///```
#[macro_export]
macro_rules! execute {
	($thread_pool:expr, $program:expr) => {{
		$thread_pool.execute(async move { $program }, bmw_deps::rand::random())
	}};
	($thread_pool:expr, $id:expr, $program:expr) => {{
		$thread_pool.execute(async move { $program }, $id)
	}};
}

/// This macro causes the current thread to block until the specified thread execution completes. Upon
/// completion, the [`crate::PoolResult`] is returned.
///
/// # Input Parameters
/// thread_pool_handle ([`crate::ThreadPoolHandle`]) (required) - The [`crate::ThreadPoolHandle`]
/// to block on.
///
/// # Return
/// Returns [`crate::PoolResult`] which contains the Ok or Err value returned by this task.
///
/// # Errors
/// * [`bmw_err::ErrKind::ThreadPanic`] - if the underlying task results in a thread panic. This
/// error is returned in the [`crate::PoolResult::Err`] variant.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///         let mut tp = thread_pool!()?;
///
///         tp.set_on_panic(move |id, e| -> Result<(), Error> {
///             let e = e.downcast_ref::<&str>().unwrap_or(&"unknown panic type");
///             info!("PANIC: id={},e={}", id, e)?;
///             Ok(())
///         })?;
///
///         tp.start()?;
///
///         let tph = execute!(tp, {
///             info!("executing a task in another thread!")?;
///             Ok(101)
///         })?;
///
///         let res = block_on!(tph);
///         assert_eq!(res, PoolResult::Ok(101));
///
///         Ok(())
/// }
///
///```
#[macro_export]
macro_rules! block_on {
	($res:expr) => {{
		$res.block_on()
	}};
}
