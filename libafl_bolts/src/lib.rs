//! Bolts are no conceptual fuzzing elements, but they keep libafl-based fuzzers together.

/*!
Welcome to `LibAFL`
*/

#![allow(incomplete_features)]
#![no_std]
// For `type_eq`
#![cfg_attr(unstable_feature, feature(specialization))]
// For `type_id` and owned things
#![cfg_attr(unstable_feature, feature(intrinsics))]
// For `std::simd`
#![cfg_attr(unstable_feature, feature(portable_simd))]
#![warn(clippy::cargo)]
#![allow(ambiguous_glob_reexports)]
#![deny(clippy::cargo_common_metadata)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(
    clippy::unreadable_literal,
    clippy::type_repetition_in_bounds,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation,
    clippy::used_underscore_binding,
    clippy::ptr_as_ptr,
    clippy::missing_panics_doc,
    clippy::missing_docs_in_private_items,
    clippy::module_name_repetitions,
    clippy::ptr_cast_constness
)]
#![cfg_attr(not(test), warn(
    missing_debug_implementations,
    missing_docs,
    //trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    //unused_results
))]
#![cfg_attr(test, deny(
    missing_debug_implementations,
    missing_docs,
    //trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_must_use,
    //unused_results
))]
#![cfg_attr(
    test,
    deny(
        bad_style,
        dead_code,
        improper_ctypes,
        non_shorthand_field_patterns,
        no_mangle_generic_items,
        overflowing_literals,
        path_statements,
        patterns_in_fns_without_body,
        private_in_public,
        unconditional_recursion,
        unused,
        unused_allocation,
        unused_comparisons,
        unused_parens,
        while_true
    )
)]
// Till they fix this buggy lint in clippy
#![allow(clippy::borrow_as_ptr)]
#![allow(clippy::borrow_deref_ref)]

#[cfg(feature = "std")]
#[macro_use]
extern crate std;
#[macro_use]
#[doc(hidden)]
pub extern crate alloc;
#[cfg(feature = "ctor")]
#[doc(hidden)]
pub use ctor::ctor;

#[deprecated(
    since = "0.11.0",
    note = "The launcher module has moved out of `libafl_bolts` into `libafl::events::launcher`."
)]
/// Dummy module informing potential users that the launcher module has moved
/// out of `libafl_bolts` into `libafl::events::launcher`.
pub mod launcher {}

// Re-export derive(SerdeAny)
#[cfg(feature = "libafl_derive")]
#[allow(unused_imports)]
#[macro_use]
extern crate libafl_derive;
use alloc::string::{FromUtf8Error, String};
use core::{
    array::TryFromSliceError,
    fmt,
    num::{ParseIntError, TryFromIntError},
};
#[cfg(feature = "std")]
use std::{env::VarError, io};

#[cfg(feature = "libafl_derive")]
pub use libafl_derive::SerdeAny;

/// We need fixed names for many parts of this lib.
pub trait Named {
    /// Provide the name of this element.
    fn name(&self) -> &str;
}

#[cfg(feature = "errors_backtrace")]
/// Error Backtrace type when `errors_backtrace` feature is enabled (== [`backtrace::Backtrace`])
pub type ErrorBacktrace = backtrace::Backtrace;

#[cfg(not(feature = "errors_backtrace"))]
#[derive(Debug, Default)]
/// Empty struct to use when `errors_backtrace` is disabled
pub struct ErrorBacktrace {}
#[cfg(not(feature = "errors_backtrace"))]
impl ErrorBacktrace {
    /// Nop
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(feature = "errors_backtrace")]
fn display_error_backtrace(f: &mut fmt::Formatter, err: &ErrorBacktrace) -> fmt::Result {
    write!(f, "\nBacktrace: {err:?}")
}
#[cfg(not(feature = "errors_backtrace"))]
#[allow(clippy::unnecessary_wraps)]
fn display_error_backtrace(_f: &mut fmt::Formatter, _err: &ErrorBacktrace) -> fmt::Result {
    fmt::Result::Ok(())
}

/// Main error struct for `LibAFL`
#[derive(Debug)]
pub enum Error {
    /// Serialization error
    Serialize(String, ErrorBacktrace),
    /// Compression error
    #[cfg(feature = "llmp_compression")]
    Compression(ErrorBacktrace),
    /// File related error
    #[cfg(feature = "std")]
    File(io::Error, ErrorBacktrace),
    /// Optional val was supposed to be set, but isn't.
    EmptyOptional(String, ErrorBacktrace),
    /// Key not in Map
    KeyNotFound(String, ErrorBacktrace),
    /// No elements in the current item
    Empty(String, ErrorBacktrace),
    /// End of iteration
    IteratorEnd(String, ErrorBacktrace),
    /// This is not supported (yet)
    NotImplemented(String, ErrorBacktrace),
    /// You're holding it wrong
    IllegalState(String, ErrorBacktrace),
    /// The argument passed to this method or function is not valid
    IllegalArgument(String, ErrorBacktrace),
    /// The performed action is not supported on the current platform
    Unsupported(String, ErrorBacktrace),
    /// Shutting down, not really an error.
    ShuttingDown,
    /// Something else happened
    Unknown(String, ErrorBacktrace),
}

impl Error {
    /// Serialization error
    #[must_use]
    pub fn serialize<S>(arg: S) -> Self
    where
        S: Into<String>,
    {
        Error::Serialize(arg.into(), ErrorBacktrace::new())
    }
    #[cfg(feature = "llmp_compression")]
    /// Compression error
    #[must_use]
    pub fn compression() -> Self {
        Error::Compression(ErrorBacktrace::new())
    }
    #[cfg(feature = "std")]
    /// File related error
    #[must_use]
    pub fn file(arg: io::Error) -> Self {
        Error::File(arg, ErrorBacktrace::new())
    }
    /// Optional val was supposed to be set, but isn't.
    #[must_use]
    pub fn empty_optional<S>(arg: S) -> Self
    where
        S: Into<String>,
    {
        Error::EmptyOptional(arg.into(), ErrorBacktrace::new())
    }
    /// Key not in Map
    #[must_use]
    pub fn key_not_found<S>(arg: S) -> Self
    where
        S: Into<String>,
    {
        Error::KeyNotFound(arg.into(), ErrorBacktrace::new())
    }
    /// No elements in the current item
    #[must_use]
    pub fn empty<S>(arg: S) -> Self
    where
        S: Into<String>,
    {
        Error::Empty(arg.into(), ErrorBacktrace::new())
    }
    /// End of iteration
    #[must_use]
    pub fn iterator_end<S>(arg: S) -> Self
    where
        S: Into<String>,
    {
        Error::IteratorEnd(arg.into(), ErrorBacktrace::new())
    }
    /// This is not supported (yet)
    #[must_use]
    pub fn not_implemented<S>(arg: S) -> Self
    where
        S: Into<String>,
    {
        Error::NotImplemented(arg.into(), ErrorBacktrace::new())
    }
    /// You're holding it wrong
    #[must_use]
    pub fn illegal_state<S>(arg: S) -> Self
    where
        S: Into<String>,
    {
        Error::IllegalState(arg.into(), ErrorBacktrace::new())
    }
    /// The argument passed to this method or function is not valid
    #[must_use]
    pub fn illegal_argument<S>(arg: S) -> Self
    where
        S: Into<String>,
    {
        Error::IllegalArgument(arg.into(), ErrorBacktrace::new())
    }
    /// Shutting down, not really an error.
    #[must_use]
    pub fn shutting_down() -> Self {
        Error::ShuttingDown
    }
    /// This operation is not supported on the current architecture or platform
    #[must_use]
    pub fn unsupported<S>(arg: S) -> Self
    where
        S: Into<String>,
    {
        Error::Unsupported(arg.into(), ErrorBacktrace::new())
    }
    /// Something else happened
    #[must_use]
    pub fn unknown<S>(arg: S) -> Self
    where
        S: Into<String>,
    {
        Error::Unknown(arg.into(), ErrorBacktrace::new())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Serialize(s, b) => {
                write!(f, "Error in Serialization: `{0}`", &s)?;
                display_error_backtrace(f, b)
            }
            #[cfg(feature = "llmp_compression")]
            Self::Compression(b) => {
                write!(f, "Error in decompression")?;
                display_error_backtrace(f, b)
            }
            #[cfg(feature = "std")]
            Self::File(err, b) => {
                write!(f, "File IO failed: {:?}", &err)?;
                display_error_backtrace(f, b)
            }
            Self::EmptyOptional(s, b) => {
                write!(f, "Optional value `{0}` was not set", &s)?;
                display_error_backtrace(f, b)
            }
            Self::KeyNotFound(s, b) => {
                write!(f, "Key `{0}` not in Corpus", &s)?;
                display_error_backtrace(f, b)
            }
            Self::Empty(s, b) => {
                write!(f, "No items in {0}", &s)?;
                display_error_backtrace(f, b)
            }
            Self::IteratorEnd(s, b) => {
                write!(f, "All elements have been processed in {0} iterator", &s)?;
                display_error_backtrace(f, b)
            }
            Self::NotImplemented(s, b) => {
                write!(f, "Not implemented: {0}", &s)?;
                display_error_backtrace(f, b)
            }
            Self::IllegalState(s, b) => {
                write!(f, "Illegal state: {0}", &s)?;
                display_error_backtrace(f, b)
            }
            Self::IllegalArgument(s, b) => {
                write!(f, "Illegal argument: {0}", &s)?;
                display_error_backtrace(f, b)
            }
            Self::Unsupported(s, b) => {
                write!(
                    f,
                    "The operation is not supported on the current platform: {0}",
                    &s
                )?;
                display_error_backtrace(f, b)
            }
            Self::ShuttingDown => write!(f, "Shutting down!"),
            Self::Unknown(s, b) => {
                write!(f, "Unknown error: {0}", &s)?;
                display_error_backtrace(f, b)
            }
        }
    }
}

/// Stringify the postcard serializer error
impl From<postcard::Error> for Error {
    fn from(err: postcard::Error) -> Self {
        Self::serialize(format!("{err:?}"))
    }
}

/// Stringify the json serializer error
#[cfg(feature = "std")]
impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::serialize(format!("{err:?}"))
    }
}

#[cfg(all(unix, feature = "std"))]
impl From<nix::Error> for Error {
    fn from(err: nix::Error) -> Self {
        Self::unknown(format!("Unix error: {err:?}"))
    }
}

/// Create an AFL Error from io Error
#[cfg(feature = "std")]
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::file(err)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Self {
        Self::unknown(format!("Could not convert byte / utf-8: {err:?}"))
    }
}

#[cfg(feature = "std")]
impl From<VarError> for Error {
    fn from(err: VarError) -> Self {
        Self::empty(format!("Could not get env var: {err:?}"))
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Self {
        Self::unknown(format!("Failed to parse Int: {err:?}"))
    }
}

impl From<TryFromIntError> for Error {
    fn from(err: TryFromIntError) -> Self {
        Self::illegal_state(format!("Expected conversion failed: {err:?}"))
    }
}

impl From<TryFromSliceError> for Error {
    fn from(err: TryFromSliceError) -> Self {
        Self::illegal_argument(format!("Could not convert slice: {err:?}"))
    }
}

#[cfg(windows)]
impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Self {
        Self::unknown(format!("Windows API error: {err:?}"))
    }
}

#[cfg(feature = "python")]
impl From<pyo3::PyErr> for Error {
    fn from(err: pyo3::PyErr) -> Self {
        pyo3::Python::with_gil(|py| {
            if err.matches(
                py,
                pyo3::types::PyType::new::<pyo3::exceptions::PyKeyboardInterrupt>(py),
            ) {
                Self::shutting_down()
            } else {
                Self::illegal_state(format!("Python exception: {err:?}"))
            }
        })
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

/// The purpose of this module is to alleviate imports of many components by adding a glob import.
#[cfg(feature = "prelude")]
pub mod prelude {
    pub use super::{bolts_prelude::*, *};
}

#[cfg(all(any(doctest, test), not(feature = "std")))]
/// Provide custom time in `no_std` tests.
#[no_mangle]
pub unsafe extern "C" fn external_current_millis() -> u64 {
    // TODO: use "real" time here
    1000
}

pub mod anymap;
#[cfg(feature = "std")]
pub mod build_id;
#[cfg(all(
    any(feature = "cli", feature = "frida_cli", feature = "qemu_cli"),
    feature = "std"
))]
pub mod cli;
#[cfg(feature = "llmp_compression")]
pub mod compress;
#[cfg(feature = "std")]
pub mod core_affinity;
pub mod cpu;
#[cfg(feature = "std")]
pub mod fs;
pub mod llmp;
#[cfg(all(feature = "std", unix))]
pub mod minibsod;
pub mod os;
pub mod ownedref;
pub mod rands;
pub mod serdeany;
pub mod shmem;
#[cfg(feature = "std")]
pub mod staterestore;
pub mod tuples;

use alloc::vec::Vec;
use core::{iter::Iterator, ops::AddAssign, time};
#[cfg(feature = "std")]
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// The client ID == the sender id.
#[repr(transparent)]
#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct ClientId(pub u32);

#[cfg(feature = "std")]
use log::{Metadata, Record};

/// Can be converted to a slice
pub trait AsSlice {
    /// Type of the entries in this slice
    type Entry;
    /// Convert to a slice
    fn as_slice(&self) -> &[Self::Entry];
}

/// Can be converted to a mutable slice
pub trait AsMutSlice {
    /// Type of the entries in this mut slice
    type Entry;
    /// Convert to a slice
    fn as_mut_slice(&mut self) -> &mut [Self::Entry];
}

impl<T> AsSlice for Vec<T> {
    type Entry = T;

    fn as_slice(&self) -> &[Self::Entry] {
        self
    }
}

impl<T> AsMutSlice for Vec<T> {
    type Entry = T;

    fn as_mut_slice(&mut self) -> &mut [Self::Entry] {
        self
    }
}

impl<T> AsSlice for &[T] {
    type Entry = T;

    fn as_slice(&self) -> &[Self::Entry] {
        self
    }
}

impl<T> AsSlice for [T] {
    type Entry = T;

    fn as_slice(&self) -> &[Self::Entry] {
        self
    }
}

impl<T> AsMutSlice for &mut [T] {
    type Entry = T;

    fn as_mut_slice(&mut self) -> &mut [Self::Entry] {
        self
    }
}

impl<T> AsMutSlice for [T] {
    type Entry = T;

    fn as_mut_slice(&mut self) -> &mut [Self::Entry] {
        self
    }
}

/// Create an `Iterator` from a reference
pub trait AsIter<'it> {
    /// The item type
    type Item: 'it;
    /// The iterator type
    type IntoIter: Iterator<Item = &'it Self::Item>;

    /// Create an iterator from &self
    fn as_iter(&'it self) -> Self::IntoIter;
}

/// Create an `Iterator` from a mutable reference
pub trait AsIterMut<'it> {
    /// The item type
    type Item: 'it;
    /// The iterator type
    type IntoIter: Iterator<Item = &'it mut Self::Item>;

    /// Create an iterator from &mut self
    fn as_iter_mut(&'it mut self) -> Self::IntoIter;
}

/// Has a length field
pub trait HasLen {
    /// The length
    fn len(&self) -> usize;

    /// Returns `true` if it has no elements.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Has a ref count
pub trait HasRefCnt {
    /// The ref count
    fn refcnt(&self) -> isize;
    /// The ref count, mutable
    fn refcnt_mut(&mut self) -> &mut isize;
}

/// Trait to truncate slices and maps to a new size
pub trait Truncate {
    /// Reduce the size of the slice
    fn truncate(&mut self, len: usize);
}

/// Current time
#[cfg(feature = "std")]
#[must_use]
#[inline]
pub fn current_time() -> time::Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
}

// external defined function in case of `no_std`
//
// Define your own `external_current_millis()` function via `extern "C"`
// which is linked into the binary and called from here.
#[cfg(all(not(any(doctest, test)), not(feature = "std")))]
extern "C" {
    //#[no_mangle]
    fn external_current_millis() -> u64;
}

/// Current time (fixed fallback for `no_std`)
#[cfg(not(feature = "std"))]
#[inline]
#[must_use]
pub fn current_time() -> time::Duration {
    let millis = unsafe { external_current_millis() };
    time::Duration::from_millis(millis)
}

/// Given a u64 number, return a hashed number using this mixing function
/// This function is used to hash an address into a more random number (used in `libafl_frida`).
/// Mixing function: <http://mostlymangling.blogspot.com/2018/07/on-mixing-functions-in-fast-splittable.html>
#[inline]
#[must_use]
pub fn xxh3_rrmxmx_mixer(v: u64) -> u64 {
    let tmp = (v >> 32) + ((v & 0xffffffff) << 32);
    let bitflip = 0x1cad21f72c81017c ^ 0xdb979082e96dd4de;
    let mut h64 = tmp ^ bitflip;
    h64 = h64.rotate_left(49) & h64.rotate_left(24);
    h64 = h64.wrapping_mul(0x9FB21C651E98DF25);
    h64 ^= (h64 >> 35) + 8;
    h64 = h64.wrapping_mul(0x9FB21C651E98DF25);
    h64 ^= h64 >> 28;
    h64
}

/// Gets current nanoseconds since [`UNIX_EPOCH`]
#[must_use]
#[inline]
pub fn current_nanos() -> u64 {
    current_time().as_nanos() as u64
}

/// Gets current milliseconds since [`UNIX_EPOCH`]
#[must_use]
#[inline]
pub fn current_milliseconds() -> u64 {
    current_time().as_millis() as u64
}

/// Format a `Duration` into a HMS string
#[must_use]
pub fn format_duration_hms(duration: &time::Duration) -> String {
    let secs = duration.as_secs();
    format!("{}h-{}m-{}s", (secs / 60) / 60, (secs / 60) % 60, secs % 60)
}

/// Calculates the cumulative sum for a slice, in-place.
/// The values are useful for example for cumulative probabilities.
///
/// So, to give an example:
/// ```rust
/// # extern crate libafl_bolts;
/// use libafl_bolts::calculate_cumulative_sum_in_place;
///
/// let mut value = [2, 4, 1, 3];
/// calculate_cumulative_sum_in_place(&mut value);
/// assert_eq!(&[2, 6, 7, 10], &value);
/// ```
pub fn calculate_cumulative_sum_in_place<T>(mut_slice: &mut [T])
where
    T: Default + AddAssign<T> + Copy,
{
    let mut acc = T::default();

    for val in mut_slice {
        acc += *val;
        *val = acc;
    }
}

/// Stderr logger
#[cfg(feature = "std")]
pub static LIBAFL_STDERR_LOGGER: SimpleStderrLogger = SimpleStderrLogger::new();

/// Stdout logger
#[cfg(feature = "std")]
pub static LIBAFL_STDOUT_LOGGER: SimpleStdoutLogger = SimpleStdoutLogger::new();

/// A simple logger struct that logs to stderr when used with [`log::set_logger`].
#[derive(Debug)]
#[cfg(feature = "std")]
pub struct SimpleStdoutLogger {}

#[cfg(feature = "std")]
impl Default for SimpleStdoutLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "std")]
impl SimpleStdoutLogger {
    /// Create a new [`log::Log`] logger that will wrte log to stdout
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// register stdout logger
    pub fn set_logger() -> Result<(), Error> {
        log::set_logger(&LIBAFL_STDOUT_LOGGER)
            .map_err(|_| Error::unknown("Failed to register logger"))
    }
}

#[cfg(feature = "std")]
impl log::Log for SimpleStdoutLogger {
    #[inline]
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        println!(
            "[{:?}] {}: {}",
            current_time(),
            record.level(),
            record.args()
        );
    }

    fn flush(&self) {}
}

/// A simple logger struct that logs to stderr when used with [`log::set_logger`].
#[derive(Debug)]
#[cfg(feature = "std")]
pub struct SimpleStderrLogger {}

#[cfg(feature = "std")]
impl Default for SimpleStderrLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "std")]
impl SimpleStderrLogger {
    /// Create a new [`log::Log`] logger that will wrte log to stdout
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// register stderr logger
    pub fn set_logger() -> Result<(), Error> {
        log::set_logger(&LIBAFL_STDERR_LOGGER)
            .map_err(|_| Error::unknown("Failed to register logger"))
    }
}

#[cfg(feature = "std")]
impl log::Log for SimpleStderrLogger {
    #[inline]
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        eprintln!(
            "[{:?}] {}: {}",
            current_time(),
            record.level(),
            record.args()
        );
    }

    fn flush(&self) {}
}
/// The purpose of this module is to alleviate imports of the bolts by adding a glob import.
#[cfg(feature = "prelude")]
pub mod bolts_prelude {
    #[cfg(feature = "std")]
    pub use super::build_id::*;
    #[cfg(all(
        any(feature = "cli", feature = "frida_cli", feature = "qemu_cli"),
        feature = "std"
    ))]
    pub use super::cli::*;
    #[cfg(feature = "llmp_compression")]
    pub use super::compress::*;
    #[cfg(feature = "std")]
    pub use super::core_affinity::*;
    #[cfg(feature = "std")]
    pub use super::fs::*;
    #[cfg(all(feature = "std", unix))]
    pub use super::minibsod::*;
    #[cfg(feature = "std")]
    pub use super::staterestore::*;
    pub use super::{
        anymap::*, cpu::*, llmp::*, os::*, ownedref::*, rands::*, serdeany::*, shmem::*, tuples::*,
    };
}

#[cfg(feature = "python")]
#[allow(missing_docs)]
pub mod pybind {

    use pyo3::{pymodule, types::PyModule, PyResult, Python};

    #[macro_export]
    macro_rules! unwrap_me_body {
        ($wrapper:expr, $name:ident, $body:block, $wrapper_type:ident, { $($wrapper_option:tt),* }) => {
            match &$wrapper {
                $(
                    $wrapper_type::$wrapper_option(py_wrapper) => {
                        Python::with_gil(|py| -> PyResult<_> {
                            let borrowed = py_wrapper.borrow(py);
                            let $name = &borrowed.inner;
                            Ok($body)
                        })
                        .unwrap()
                    }
                )*
            }
        };
        ($wrapper:expr, $name:ident, $body:block, $wrapper_type:ident, { $($wrapper_option:tt),* }, { $($wrapper_optional:tt($pw:ident) => $code_block:block)* }) => {
            match &$wrapper {
                $(
                    $wrapper_type::$wrapper_option(py_wrapper) => {
                        Python::with_gil(|py| -> PyResult<_> {
                            let borrowed = py_wrapper.borrow(py);
                            let $name = &borrowed.inner;
                            Ok($body)
                        })
                        .unwrap()
                    }
                )*
                $($wrapper_type::$wrapper_optional($pw) => { $code_block })*
            }
        };
    }

    #[macro_export]
    macro_rules! unwrap_me_mut_body {
        ($wrapper:expr, $name:ident, $body:block, $wrapper_type:ident, { $($wrapper_option:tt),*}) => {
            match &mut $wrapper {
                $(
                    $wrapper_type::$wrapper_option(py_wrapper) => {
                        Python::with_gil(|py| -> PyResult<_> {
                            let mut borrowed = py_wrapper.borrow_mut(py);
                            let $name = &mut borrowed.inner;
                            Ok($body)
                        })
                        .unwrap()
                    }
                )*
            }
        };
        ($wrapper:expr, $name:ident, $body:block, $wrapper_type:ident, { $($wrapper_option:tt),*}, { $($wrapper_optional:tt($pw:ident) => $code_block:block)* }) => {
            match &mut $wrapper {
                $(
                    $wrapper_type::$wrapper_option(py_wrapper) => {
                        Python::with_gil(|py| -> PyResult<_> {
                            let mut borrowed = py_wrapper.borrow_mut(py);
                            let $name = &mut borrowed.inner;
                            Ok($body)
                        })
                        .unwrap()
                    }
                )*
                $($wrapper_type::$wrapper_optional($pw) => { $code_block })*
            }
        };
    }

    #[macro_export]
    macro_rules! impl_serde_pyobjectwrapper {
        ($struct_name:ident, $inner:tt) => {
            const _: () = {
                use alloc::vec::Vec;

                use pyo3::prelude::*;
                use serde::{Deserialize, Deserializer, Serialize, Serializer};

                impl Serialize for $struct_name {
                    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where
                        S: Serializer,
                    {
                        let buf = Python::with_gil(|py| -> PyResult<Vec<u8>> {
                            let pickle = PyModule::import(py, "pickle")?;
                            let buf: Vec<u8> =
                                pickle.getattr("dumps")?.call1((&self.$inner,))?.extract()?;
                            Ok(buf)
                        })
                        .unwrap();
                        serializer.serialize_bytes(&buf)
                    }
                }

                struct PyObjectVisitor;

                impl<'de> serde::de::Visitor<'de> for PyObjectVisitor {
                    type Value = $struct_name;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter
                            .write_str("Expecting some bytes to deserialize from the Python side")
                    }

                    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        let obj = Python::with_gil(|py| -> PyResult<PyObject> {
                            let pickle = PyModule::import(py, "pickle")?;
                            let obj = pickle.getattr("loads")?.call1((v,))?.to_object(py);
                            Ok(obj)
                        })
                        .unwrap();
                        Ok($struct_name::new(obj))
                    }
                }

                impl<'de> Deserialize<'de> for $struct_name {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        deserializer.deserialize_byte_buf(PyObjectVisitor)
                    }
                }
            };
        };
    }

    #[pymodule]
    #[pyo3(name = "libafl_bolts")]
    /// Register the classes to the python module
    pub fn python_module(py: Python, m: &PyModule) -> PyResult<()> {
        crate::rands::pybind::register(py, m)?;
        Ok(())
    }
}