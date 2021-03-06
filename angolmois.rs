/*
 * Angolmois -- the simple BMS player
 * Copyright (c) 2005, 2007, 2009, 2012, 2013, 2014, Kang Seonghoon.
 * Project Angolmois is copyright (c) 2003-2007, Choi Kaya (CHKY).
 * 
 * This program is free software; you can redistribute it and/or
 * modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation; either version 2
 * of the License, or (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 59 Temple Place - Suite 330, Boston, MA  02111-1307, USA.
 */

/*!
 * This is a direct, one-to-one translation of Angolmois to Rust programming language.
 * [Angolmois](http://mearie.org/projects/angolmois/) is
 * a [BM98](http://bm98.yaneu.com/bm98/)-like minimalistic music video game which supports
 * the [BMS format](http://en.wikipedia.org/wiki/Be-Music_Source) for playing.
 * 
 * Angolmois is a combination of string parsing, path manipulation, two-dimensional graphics and
 * complex game play carefully packed into some thousand lines of code. This translation is intended
 * to provide an example of translating a moderately-sized C code to Rust, and also to investigate
 * additional library supports required for such moderately-sized programs.
 * 
 * Angolmois is distributed under GNU GPL version 2+, so is this translation. The portions of it is
 * intended to be sent as a patch to Rust, so those portions are licensed under Apache License 2.0
 * and MIT license. Unlike the original Angolmois code (which sacrifices most comments due to code
 * size concerns), the Rust version has much more comments which can be beneficial for understanding
 * Angolmois itself too.
 *
 * Starting from Rust 0.9, Angolmois Rust edition tracks the most recent development version of
 * Rust. Consequently it is now synchronized with the up-to-date version of rust-sdl.
 *
 * # Key
 * 
 * The following notations are used in the comments and documentations.
 * 
 * * (C: ...) - variable/function corresponds to given name in the C code.
 * * Rust: ... - suboptimal translation with a room for improvement in Rust. often contains a Rust
 *   issue number like #1234.
 * * XXX - should be fixed as soon as Rust issue is gone.
 * * TODO - other problems unrelated to Rust.
 */

#![crate_name = "angolmois"]
#![crate_type = "bin"]

#![feature(macro_rules)]

// XXX temporarily cope with the nightly
#![allow(unknown_features)]
#![feature(slicing_syntax)]

#![comment = "Angolmois"]
#![license = "GPLv2+"]

extern crate libc;

extern crate sdl;
extern crate sdl_mixer;
extern crate sdl_image;

/// Returns a version string. (C: `VERSION`)
pub fn version() -> String { "Angolmois 2.0.0 alpha 2 (rust edition)".to_string() }

//==================================================================================================
// utility declarations

/// Returns an executable name used in the command line if any. (C: `argv0`)
pub fn exename() -> String {
    let args = std::os::args();
    if args.is_empty() {"angolmois".to_string()} else {args[0].clone()}
}

/// Utility functions.
#[macro_escape]
pub mod util {
    use std;
    use libc;

    /// String utilities for Rust. Parallels to `std::str`.
    pub mod str {
        /// Extensions to `str`.
        pub trait StrUtil<'r> {
            /// Returns a slice of the given string starting from `begin` and up to the byte
            /// position `end`. `end` doesn't have to point to valid characters.
            ///
            /// # Failure
            ///
            /// If `begin` does not point to valid characters or beyond the last character of
            /// the string, or `end` points beyond the last character of the string
            fn slice_upto(&self, begin: uint, end: uint) -> &'r str;

            /// Counts the number of bytes in the complete UTF-8 sequences up to `limit` bytes
            /// in `s` starting from `start`.
            fn count_bytes_upto(&self, start: uint, limit: uint) -> uint;

            /// Work with a null-terminated UTF-16 buffer of the string. Useful for calling
            /// Win32 API.
            fn as_utf16_c_str<T>(&self, f: |*const u16| -> T) -> T;
        }

        impl<'r> StrUtil<'r> for &'r str {
            fn slice_upto(&self, begin: uint, end: uint) -> &'r str {
                (*self)[begin..begin + self.count_bytes_upto(begin, end)]
            }

            fn count_bytes_upto(&self, start: uint, limit: uint) -> uint {
                assert!(self.is_char_boundary(start));
                let limit = start + limit;
                let l = self.len();
                assert!(limit < l);
                let mut end = start;
                loop {
                    assert!(end < l);
                    let next = self.char_range_at(end).next;
                    if next > limit { break; }
                    end = next;
                }
                end - start
            }

            fn as_utf16_c_str<T>(&self, f: |*const u16| -> T) -> T {
                let mut s16: Vec<u16> = self.utf16_units().collect();
                s16.push(0u16);
                f(s16.as_ptr())
            }
        }

        /// A version of `std::from_str::FromStr` which parses a prefix of the input and
        /// returns a remaining portion of the input as well.
        //
        // Rust: `std::num::from_str_bytes_common` does not recognize a number followed
        //        by garbage, so we need to parse it ourselves.
        pub trait FromStrPrefix {
            /// Returns a parsed value and remaining string slice if possible.
            fn from_str_prefix<'a>(s: &'a str) -> Option<(Self, &'a str)>;
        }

        /// A convenience function that invokes `FromStrPrefix::from_str_prefix`.
        pub fn from_str_prefix<'a, T:FromStrPrefix>(s: &'a str) -> Option<(T, &'a str)> {
            FromStrPrefix::from_str_prefix(s)
        }

        /// Returns a length of the longest prefix of given string,
        /// which `from_str::<uint>` would in general accept without a failure, if any.
        fn scan_uint(s: &str) -> Option<uint> {
            match s.find(|c| !('0' <= c && c <= '9')) {
                Some(first) if first > 0u => Some(first),
                None if s.len() > 0u => Some(s.len()),
                _ => None
            }
        }

        /// Returns a length of the longest prefix of given string,
        /// which `from_str::<int>` would in general accept without a failure, if any.
        fn scan_int(s: &str) -> Option<uint> {
            match s.slice_shift_char() {
                (Some('-'), s_) | (Some('+'), s_) => scan_uint(s_).map(|pos| pos + 1u),
                _ => scan_uint(s)
            }
        }

        /// Returns a length of the longest prefix of given string,
        /// which `from_str::<f64>` (and so on) would in general accept without a failure, if any.
        fn scan_float(s: &str) -> Option<uint> {
            scan_int(s).and_then(|pos| {
                // scan `.` followed by digits if any
                match s[pos..].slice_shift_char() {
                    (Some('.'), s_) => scan_uint(s_).map(|pos2| pos + 1u + pos2),
                    _ => Some(pos)
                }
            }).and_then(|pos| {
                // scan `e` or `E` followed by an optional sign and digits if any
                match s[pos..].slice_shift_char() {
                    (Some('e'), s_) | (Some('E'), s_) => scan_int(s_).map(|pos2| pos + 1u + pos2),
                    _ => Some(pos)
                }
            })
        }

        macro_rules! from_str_prefix_impls(
            ($($scan:ident then $t:ty;)*) => (
                $(
                    impl FromStrPrefix for $t {
                        fn from_str_prefix<'a>(s: &'a str) -> Option<($t, &'a str)> {
                            $scan(s).and_then(|pos| {
                                from_str::<$t>(s[..pos]).map(|v| (v, s[pos..]))
                            })
                        }
                    }
                )*
            )
        )

        from_str_prefix_impls! {
            scan_int   then int;
            scan_int   then i8;
            scan_int   then i16;
            scan_int   then i32;
            scan_int   then i64;
            scan_uint  then uint;
            scan_uint  then u8;
            scan_uint  then u16;
            scan_uint  then u32;
            scan_uint  then u64;
            scan_float then f32;
            scan_float then f64;
        }

        impl FromStrPrefix for char {
            fn from_str_prefix<'a>(s: &'a str) -> Option<(char, &'a str)> {
                match s.slice_shift_char() {
                    (Some(c), s_) => Some((c, s_)),
                    (None, _) => None,
                }
            }
        }

        /// A trait which provides `prefix_shifted` method. Similar to `str::starts_with`, but with
        /// swapped `self` and argument.
        pub trait ShiftablePrefix {
            /// When given string starts with `self`, returns a slice of that string
            /// excluding that prefix. Otherwise returns `None`.
            fn prefix_shifted<'r>(&self, s: &'r str) -> Option<&'r str>;
        }

        /// A convenience function that invokes `ShiftablePrefix::prefix_shifted`.
        pub fn prefix_shifted<'a, T:ShiftablePrefix>(s: &'a str, prefix: T) -> Option<&'a str> {
            prefix.prefix_shifted(s)
        }

        impl ShiftablePrefix for char {
            fn prefix_shifted<'r>(&self, s: &'r str) -> Option<&'r str> {
                match s.slice_shift_char() {
                    (Some(c), s_) if c == *self => Some(s_),
                    (_, _) => None,
                }
            }
        }

        impl<'r> ShiftablePrefix for &'r str {
            fn prefix_shifted<'r>(&self, s: &'r str) -> Option<&'r str> {
                if s.starts_with(*self) {
                    Some(s[self.len()..])
                } else {
                    None
                }
            }
        }
    }

    /// Option utilities for Rust. Parallels to `std::option`.
    pub mod option {
        /// An utility trait for an option of string or alikes.
        pub trait StrOption {
            /// Returns a string slice in the option if any.
            fn as_ref_slice<'a>(&'a self) -> Option<&'a str>;

            /// Returns a string slice in the option if any, or `default` otherwise.
            fn as_ref_slice_or<'a>(&'a self, default: &'a str) -> &'a str {
                self.as_ref_slice().unwrap_or(default)
            }
        }

        impl<T:Str> StrOption for Option<T> {
            fn as_ref_slice<'a>(&'a self) -> Option<&'a str> {
                self.as_ref().map(|s| s.as_slice())
            }
        }
    }

    /**
     * A minimal but functional binding for SMPEG.
     *
     * NOTE: Some of these additions will be eventually sent to rust-sdl and are not subject to
     * the above copyright notice.
     */
    pub mod smpeg {
        #![allow(non_camel_case_types)]

        use std;
        use libc::{c_int, c_float};
        use std::ptr::null_mut;
        use sdl::video::Surface;
        use self::ll::SMPEGstatus;

        pub mod ll {
            use libc::{c_void, c_int, c_char, c_float, c_double};
            use sdl::video::ll::{SDL_RWops, SDL_Surface};
            use sdl::audio::ll::SDL_AudioSpec;
            #[repr(C)]
            pub struct SMPEG { _opaque: () }
            #[repr(C)]
            pub struct SMPEG_Info {
                pub has_audio: c_int,
                pub has_video: c_int,
                pub width: c_int,
                pub height: c_int,
                pub current_frame: c_int,
                pub current_fps: c_double,
                pub audio_string: [c_char, ..80],
                pub audio_current_frame: c_int,
                pub current_offset: u32,
                pub total_size: u32,
                pub current_time: c_double,
                pub total_time: c_double
            }
            #[deriving(PartialEq, Eq, Clone)]
            #[repr(C)]
            pub enum SMPEGstatus {
                SMPEG_ERROR = -1,
                SMPEG_STOPPED = 0,
                SMPEG_PLAYING =1
            }
            #[link(name = "smpeg")]
            extern {
                pub fn SMPEG_new(file: *const c_char, info: *mut SMPEG_Info,
                                 sdl_audio: c_int) -> *mut SMPEG;
                pub fn SMPEG_new_descr(file: c_int, info: *mut SMPEG_Info,
                                       sdl_audio: c_int) -> *mut SMPEG;
                pub fn SMPEG_new_data(data: *mut c_void, size: c_int, info: *mut SMPEG_Info,
                                      sdl_audio: c_int) -> *mut SMPEG;
                pub fn SMPEG_new_rwops(src: *mut SDL_RWops, info: *mut SMPEG_Info,
                                       sdl_audio: c_int) -> *mut SMPEG;
                pub fn SMPEG_getinfo(mpeg: *mut SMPEG, info: *mut SMPEG_Info);
                pub fn SMPEG_enableaudio(mpeg: *mut SMPEG, enable: c_int);
                pub fn SMPEG_enablevideo(mpeg: *mut SMPEG, enable: c_int);
                pub fn SMPEG_delete(mpeg: *mut SMPEG);
                pub fn SMPEG_status(mpeg: *mut SMPEG) -> SMPEGstatus;
                pub fn SMPEG_setvolume(mpeg: *mut SMPEG, volume: c_int);
                // XXX SDL_Mutex and SMPEG_DisplayCallback unimplemented
                pub fn SMPEG_setdisplay(mpeg: *mut SMPEG, dst: *mut SDL_Surface,
                                        surfLock: *mut c_void, callback: *mut c_void);
                pub fn SMPEG_loop(mpeg: *mut SMPEG, repeat: c_int);
                pub fn SMPEG_scaleXY(mpeg: *mut SMPEG, width: c_int, height: c_int);
                pub fn SMPEG_scale(mpeg: *mut SMPEG, scale: c_int);
                pub fn SMPEG_move(mpeg: *mut SMPEG, x: c_int, y: c_int);
                pub fn SMPEG_setdisplayregion(mpeg: *mut SMPEG, x: c_int, y: c_int,
                                              w: c_int, h: c_int);
                pub fn SMPEG_play(mpeg: *mut SMPEG);
                pub fn SMPEG_pause(mpeg: *mut SMPEG);
                pub fn SMPEG_stop(mpeg: *mut SMPEG);
                pub fn SMPEG_rewind(mpeg: *mut SMPEG);
                pub fn SMPEG_seek(mpeg: *mut SMPEG, bytes: c_int);
                pub fn SMPEG_skip(mpeg: *mut SMPEG, seconds: c_float);
                pub fn SMPEG_renderFrame(mpeg: *mut SMPEG, framenum: c_int);
                pub fn SMPEG_renderFinal(mpeg: *mut SMPEG, dst: *mut SDL_Surface,
                                         x: c_int, y: c_int);
                // XXX SMPEG_Filter unimplemented
                pub fn SMPEG_filter(mpeg: *mut SMPEG, filter: *mut c_void) -> *mut c_void;
                pub fn SMPEG_error(mpeg: *mut SMPEG) -> *mut c_char;
                pub fn SMPEG_playAudio(mpeg: *mut SMPEG, stream: *mut u8, len: c_int) -> c_int;
                pub fn SMPEG_playAudioSDL(mpeg: *mut c_void, stream: *mut u8, len: c_int) -> c_int;
                pub fn SMPEG_wantedSpec(mpeg: *mut SMPEG, wanted: *mut SDL_AudioSpec) -> c_int;
                pub fn SMPEG_actualSpec(mpeg: *mut SMPEG, spec: *mut SDL_AudioSpec);
            }
        }

        pub struct MPEG {
            pub raw: *mut ll::SMPEG
        }

        fn wrap_mpeg(raw: *mut ll::SMPEG) -> MPEG {
            MPEG { raw: raw }
        }

        impl Drop for MPEG {
            fn drop(&mut self) {
                unsafe { ll::SMPEG_delete(self.raw); }
            }
        }

        impl MPEG {
            pub fn from_path(path: &Path) -> Result<MPEG, String> {
                let raw = unsafe {
                    let path = path.to_c_str();
                    ll::SMPEG_new(path.as_ptr(), null_mut(), 0)
                };

                if raw.is_null() { Err(::sdl::get_error()) }
                else { Ok(wrap_mpeg(raw)) }
            }

            pub fn status(&self) -> SMPEGstatus {
                unsafe { ll::SMPEG_status(self.raw) }
            }

            pub fn set_volume(&self, volume: int) {
                unsafe { ll::SMPEG_setvolume(self.raw, volume as c_int); }
            }

            pub fn set_display(&self, surface: &Surface) {
                unsafe {
                    ll::SMPEG_setdisplay(self.raw, surface.raw, null_mut(), null_mut());
                }
            }

            pub fn enable_video(&self, enable: bool) {
                unsafe { ll::SMPEG_enablevideo(self.raw, enable as c_int); }
            }

            pub fn enable_audio(&self, enable: bool) {
                unsafe { ll::SMPEG_enableaudio(self.raw, enable as c_int); }
            }

            pub fn set_loop(&self, repeat: bool) {
                unsafe { ll::SMPEG_loop(self.raw, repeat as c_int); }
            }

            pub fn resize(&self, width: int, height: int) {
                unsafe { ll::SMPEG_scaleXY(self.raw, width as c_int, height as c_int); }
            }

            pub fn scale_by(&self, scale: int) {
                unsafe { ll::SMPEG_scale(self.raw, scale as c_int); }
            }

            pub fn move_by(&self, x: int, y: int) {
                unsafe { ll::SMPEG_move(self.raw, x as c_int, y as c_int); }
            }

            pub fn set_display_region(&self, x: int, y: int, w: int, h: int) {
                unsafe {
                    ll::SMPEG_setdisplayregion(self.raw, x as c_int, y as c_int,
                                               w as c_int, h as c_int);
                }
            }

            pub fn play(&self) {
                unsafe { ll::SMPEG_play(self.raw); }
            }

            pub fn pause(&self) {
                unsafe { ll::SMPEG_pause(self.raw); }
            }

            pub fn stop(&self) {
                unsafe { ll::SMPEG_stop(self.raw); }
            }

            pub fn rewind(&self) {
                unsafe { ll::SMPEG_rewind(self.raw); }
            }

            pub fn seek(&self, bytes: int) {
                unsafe { ll::SMPEG_seek(self.raw, bytes as c_int); }
            }

            pub fn skip(&self, seconds: f64) {
                unsafe { ll::SMPEG_skip(self.raw, seconds as c_float); }
            }

            pub fn get_error(&self) -> String {
                unsafe {
                    let cstr = ll::SMPEG_error(self.raw);
                    std::string::raw::from_buf(std::mem::transmute(&cstr))
                }
            }
        }
    }

    /// Win32 API wrappers.
    #[cfg(target_os = "windows")]
    pub mod win32 {
        pub mod ll {
            #![allow(non_camel_case_types)]

            use libc::{c_int, c_uint, c_void};
            use libc::types::os::arch::extra::{BOOL, WORD, DWORD, HANDLE, LPWSTR, LPCWSTR};

            pub type HWND = HANDLE;
            pub type HINSTANCE = HANDLE;

            pub const OFN_HIDEREADONLY: DWORD = 4;

            #[allow(non_snake_case)]
            #[repr(C)]
            pub struct OPENFILENAMEW {
                pub lStructSize: DWORD,
                pub hwndOwner: HWND,
                pub hInstance: HINSTANCE,
                pub lpstrFilter: LPCWSTR,
                pub lpstrCustomFilter: LPWSTR,
                pub nMaxCustFilter: DWORD,
                pub nFilterIndex: DWORD,
                pub lpstrFile: LPWSTR,
                pub nMaxFile: DWORD,
                pub lpstrFileTitle: LPWSTR,
                pub nMaxFileTitle: DWORD,
                pub lpstrInitialDir: LPCWSTR,
                pub lpstrTitle: LPCWSTR,
                pub Flags: DWORD,
                pub nFileOffset: WORD,
                pub nFileExtension: WORD,
                pub lpstrDefExt: LPCWSTR,
                pub lCustData: DWORD,
                pub lpfnHook: *mut (), // XXX LPOFNHOOKPROC = fn(HWND,c_uint,WPARAM,LPARAM)->c_uint
                pub lpTemplateName: LPCWSTR,
                pub pvReserved: *mut c_void,
                pub dwReserved: DWORD,
                pub FlagsEx: DWORD,
            }

            #[link(name = "user32")]
            extern "system" {
                pub fn MessageBoxW(hWnd: HWND, lpText: LPCWSTR, lpCaption: LPCWSTR,
                                   uType: c_uint) -> c_int;
            }

            #[link(name = "comdlg32")]
            extern "system" {
                pub fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> BOOL;
            }
        }
    }

    /// Immediately terminates the program with given exit code.
    pub fn exit(exitcode: int) -> ! {
        // Rust: `std::os::set_exit_status` doesn't immediately terminate the program.
        unsafe { libc::exit(exitcode as libc::c_int); }
    }

    /// Exits with an error message. Internally used in the `die!` macro below.
    #[cfg(target_os = "windows")]
    pub fn die(s: &str) -> ! {
        use util::str::StrUtil;
        ::exename()[].as_utf16_c_str(|caption| {
            s[].as_utf16_c_str(|text| {
                unsafe { win32::ll::MessageBoxW(std::ptr::null_mut(), text, caption, 0); }
            })
        });
        exit(1)
    }

    /// Exits with an error message. Internally used in the `die!` macro below.
    #[cfg(not(target_os = "windows"))]
    pub fn die(s: &str) -> ! {
        let mut stderr = std::io::stderr();
        let _ = writeln!(&mut stderr, "{}: {}", ::exename(), s);
        exit(1)
    }

    /// Prints an warning message. Internally used in the `warn!` macro below.
    pub fn warn(s: &str) {
        let mut stderr = std::io::stderr();
        let _ = writeln!(&mut stderr, "*** Warning: {}", s);
    }

    /// Exits with a formatted error message. (C: `die`)
    macro_rules! die(
        ($($e:expr),+) => (::util::die(format!($($e),+)[]))
    )

    /// Prints a formatted warning message. (C: `warn`)
    macro_rules! warn(
        ($($e:expr),+) => (::util::warn(format!($($e),+)[]))
    )

    /// Reads a path string from the user in the platform-dependent way. Returns `None` if the user
    /// refused to do so or the platform is unsupported. (C: `filedialog`)
    #[cfg(target_os = "windows")]
    pub fn get_path_from_dialog() -> Option<String> {
        use std::ptr::{null, null_mut};
        use util::str::StrUtil;

        let filter =
            "All Be-Music Source File (*.bms;*.bme;*.bml;*.pms)\x00*.bms;*.bme;*.bml;*.pms\x00\
             Be-Music Source File (*.bms)\x00*.bms\x00\
             Extended Be-Music Source File (*.bme)\x00*.bme\x00\
             Longnote Be-Music Source File (*.bml)\x00*.bml\x00\
             Po-Mu Source File (*.pms)\x00*.pms\x00\
             All Files (*.*)\x00*.*\x00";
        filter.as_utf16_c_str(|filter| {
            "Choose a file to play".as_utf16_c_str(|title| {
                let mut buf = [0u16, ..512];
                let ofnsz = std::mem::size_of::<win32::ll::OPENFILENAMEW>();
                let mut ofn = win32::ll::OPENFILENAMEW {
                    lStructSize: ofnsz as libc::DWORD,
                    lpstrFilter: filter,
                    lpstrFile: buf.as_mut_ptr(),
                    nMaxFile: buf.len() as libc::DWORD,
                    lpstrTitle: title,
                    Flags: win32::ll::OFN_HIDEREADONLY,

                    // zero-initialized fields
                    hwndOwner: null_mut(), hInstance: null_mut(),
                    lpstrCustomFilter: null_mut(), nMaxCustFilter: 0, nFilterIndex: 0,
                    lpstrFileTitle: null_mut(), nMaxFileTitle: 0,
                    lpstrInitialDir: null(), nFileOffset: 0, nFileExtension: 0,
                    lpstrDefExt: null(), lCustData: 0, lpfnHook: null_mut(),
                    lpTemplateName: null(), pvReserved: null_mut(),
                    dwReserved: 0, FlagsEx: 0,
                };
                let ret = unsafe {win32::ll::GetOpenFileNameW(&mut ofn)};
                if ret != 0 {
                    let path: &[u16] = match buf.position_elem(&0) {
                        Some(idx) => buf[..idx],
                        None => buf[]
                    };
                    String::from_utf16(path)
                } else {
                    None
                }
            })
        })
    }

    /// Reads a path string from the user in the platform-dependent way. Returns `None` if the user
    /// refused to do so or the platform is unsupported. (C: `filedialog`)
    #[cfg(not(target_os = "windows"))]
    pub fn get_path_from_dialog() -> Option<String> {
        None
    }

    /**
     * A lexer barely powerful enough to parse BMS format. Comparable to C's `sscanf`.
     *
     * `lex!(e; fmt1, fmt2, ..., fmtN)` returns an expression that evaluates to true if and only if
     * all format specification is consumed. The format specification (analogous to `sscanf`'s
     * `%`-string) is as follows:
     *
     * - `ws`: Consumes one or more whitespace. (C: `%*[ \t\r\n]` or similar)
     * - `ws*`: Consumes zero or more whitespace. (C: ` `)
     * - `int [-> e2]` and so on: Any type implementing the `FromStrPrefix` trait can be used.
     *   Optionally saves the parsed value to `e2`. The default implementations includes
     *   all integers, floating point types and `char`. (C: `%d`/`%*d`, `%f`/`%*f`, and `%1c`
     *   with slight lexical differences)
     * - `str [-> e2]`: Consumes a remaining input as a string and optionally saves it to `e2`.
     *   The string is at least one character long. (C: not really maps to `sscanf`, similar to
     *   `fgets`) Implies `!`. It can be followed by `ws*` which makes the string right-trimmed.
     * - `str* [-> e2]`: Same as above but the string can be empty.
     * - `!`: Ensures that the entire string has been consumed. Should be the last format
     *   specification.
     * - `lit "foo"`, `lit '.'` etc.: A literal string or literal character.
     *
     * Whitespaces are only trimmed when `ws` or `ws*` specification is used.
     * Therefore `char`, for example, can read a whitespace when not prepended with `ws` or `ws*`.
     */
    // Rust: - multiple statements do not expand correctly. (#4375)
    //       - it is desirable to have a matcher only accepts an integer literal or string literal,
    //         not a generic expression.
    //       - it would be more useful to generate bindings for parsed result. this is related to
    //         many issues in general.
    //       - could we elide a `lit` prefix somehow?
    macro_rules! lex(
        ($e:expr; ) => (true);
        ($e:expr; !) => ($e.is_empty());

        ($e:expr; str -> $dst:expr, ws*, $($tail:tt)*) => ({
            let _line: &str = $e;
            if !_line.is_empty() {
                $dst = _line.trim_right();
                lex!(""; $($tail)*) // optimization!
            } else {
                false
            }
        });
        ($e:expr; str -> $dst:expr, $($tail:tt)*) => ({
            let _line: &str = $e;
            if !_line.is_empty() {
                $dst = _line;
                lex!(""; $($tail)*) // optimization!
            } else {
                false
            }
        });
        ($e:expr; str* -> $dst:expr, ws*, $($tail:tt)*) => ({
            let _line: &str = $e;
            $dst = _line.trim_right();
            lex!(""; $($tail)*) // optimization!
        });
        ($e:expr; str* -> $dst:expr, $($tail:tt)*) => ({
            let _line: &str = $e;
            $dst = _line;
            lex!(""; $($tail)*) // optimization!
        });
        ($e:expr; $t:ty -> $dst:expr, $($tail:tt)*) => ({
            let _line: &str = $e;
            ::util::str::from_str_prefix::<$t>(_line).map_or(false, |(_value, _line)| {
                $dst = _value;
                lex!(_line; $($tail)*)
            })
        });

        ($e:expr; ws, $($tail:tt)*) => ({
            let _line: &str = $e;
            if !_line.is_empty() && _line.char_at(0).is_whitespace() {
                lex!(_line.trim_left(); $($tail)*)
            } else {
                false
            }
        });
        ($e:expr; ws*, $($tail:tt)*) => ({
            let _line: &str = $e;
            lex!(_line.trim_left(); $($tail)*)
        });
        ($e:expr; str, $($tail:tt)*) => ({
            !$e.is_empty() && lex!(""; $($tail)*) // optimization!
        });
        ($e:expr; str*, $($tail:tt)*) => ({
            lex!(""; $($tail)*) // optimization!
        });
        ($e:expr; $t:ty, $($tail:tt)*) => ({
            let mut _dummy: $t; // unused
            lex!($e; $t -> _dummy, $($tail)*)
        });
        ($e:expr; lit $lit:expr, $($tail:tt)*) => ({
            ::util::str::prefix_shifted($e, $lit).map_or(false, |_line| {
                lex!(_line; $($tail)*)
            })
        });

        ($e:expr; str -> $dst:expr) => (lex!($e; str -> $dst, ));
        ($e:expr; str -> $dst:expr, ws*) => (lex!($e; str -> $dst, ws*, ));
        ($e:expr; str* -> $dst:expr) => (lex!($e; str* -> $dst, ));
        ($e:expr; str* -> $dst:expr, ws*) => (lex!($e; str* -> $dst, ws*, ));
        ($e:expr; $t:ty -> $dst:expr) => (lex!($e; $t -> $dst, ));

        ($e:expr; ws) => (lex!($e; ws, ));
        ($e:expr; ws*) => (lex!($e; ws*, ));
        ($e:expr; str) => (lex!($e; str, ));
        ($e:expr; str*) => (lex!($e; str*, ));
        ($e:expr; $t:ty) => (lex!($e; $t, ));
        ($e:expr; lit $lit:expr) => (lex!($e; lit $lit, ))
    )

}

//==================================================================================================
// bms parser

/**
 * BMS parser module.
 *
 * # Structure
 *
 * The BMS format is a plain text format with most directives start with optional whitespace
 * followed by `#`. Besides the metadata (title, artist etc.), a BMS file is a map from the time
 * position to various game play elements (henceforth "objects") and other object-like effects
 * including BGM and BGA changes. It also contains preprocessor directives used to randomize some or
 * all parts of the BMS file, which would only make sense in the loading time.
 *
 * The time position is a virtual time divided by an unit of (musical) measure. It is related to
 * the actual time by the current Beats Per Minute (BPM) value which can, well, also change during
 * the game play. Consequently it is convenient to refer the position in terms of measures, which
 * the BMS format does: the lines `#xxxyy:AABBCC...` indicates that the measure number `xxx`
 * contains objects or object-like effects (of the type specified by `yy`, henceforth "channels"),
 * evenly spaced throughout the measure and which data values are `AA`, `BB`, `CC` respectively.
 *
 * An alphanumeric identifier (henceforth "alphanumeric key") like `AA` or `BB` may mean that
 * the actual numeric value interpreted as base 16 or 36 (depending on the channel), or a reference
 * to other assets (e.g. `#BMPAA foo.png`) or complex values specified by other commands (e.g.
 * `#BPMBB 192.0`). In most cases, an identifier `00` indicates an absence of objects or object-like
 * effects at that position.
 *
 * More detailed information about BMS format, including surveys about how different implementations
 * (so called BMS players) react to underspecified features or edge cases, can be found at [BMS
 * command memo](http://hitkey.nekokan.dyndns.info/cmds.htm).
 */
pub mod parser {
    use std::{f64, str, iter, io, fmt};
    use std::rand::Rng;
    use util::str::FromStrPrefix;

    //----------------------------------------------------------------------------------------------
    // alphanumeric key

    /// Two-letter alphanumeric identifier used for virtually everything, including resource
    /// management, variable BPM and chart specification.
    #[deriving(PartialEq,PartialOrd,Eq,Ord,Clone)]
    pub struct Key(pub int);

    /// The number of all possible alphanumeric keys. (C: `MAXKEY`)
    pub const MAXKEY: int = 36*36;

    impl Deref<int> for Key {
        fn deref<'a>(&'a self) -> &'a int {
            let Key(ref v) = *self;
            v
        }
    }

    impl Key {
        /// Returns if the alphanumeric key is in the proper range. Angolmois supports the full
        /// range of 00-ZZ (0-1295) for every case.
        pub fn is_valid(&self) -> bool {
            0 <= **self && **self < MAXKEY
        }

        /// Re-reads the alphanumeric key as a hexadecimal number if possible. This is required
        /// due to handling of channel #03 (BPM is expected to be in hexadecimal).
        pub fn to_hex(&self) -> Option<int> {
            let sixteens = **self / 36;
            let ones = **self % 36;
            if sixteens < 16 && ones < 16 {Some(sixteens * 16 + ones)} else {None}
        }
    }

    impl fmt::Show for Key {
        /// Returns a two-letter representation of alphanumeric key. (C: `TO_KEY`)
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            assert!(self.is_valid());
            let sixteens = **self / 36;
            let ones = **self % 36;
            let map = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
            write!(f, "{}{}", map[sixteens as uint] as char, map[ones as uint] as char)
        }
    }

    //----------------------------------------------------------------------------------------------
    // lane and key kinds

    /// A game play element mapped to the single input element (for example, button) and the screen
    /// area (henceforth "lane").
    #[deriving(PartialEq,Eq,Clone)]
    pub struct Lane(pub uint);

    /// The maximum number of lanes. (C: `NNOTECHANS`)
    pub const NLANES: uint = 72;

    impl Deref<uint> for Lane {
        fn deref<'a>(&'a self) -> &'a uint {
            let Lane(ref v) = *self;
            v
        }
    }

    impl Lane {
        /// Converts the channel number to the lane number.
        pub fn from_channel(chan: Key) -> Lane {
            let player = match *chan / 36 {
                1 | 3 | 5 | 0xD => 0,
                2 | 4 | 6 | 0xE => 1,
                _ => panic!("non-object channel")
            };
            Lane(player * 36 + *chan as uint % 36)
        }
    }

    /**
     * Key kinds. They define an appearance of particular lane, but otherwise ignored for the game
     * play. Angolmois supports several key kinds in order to cover many potential uses.
     * (C: `KEYKIND_MNEMONICS`)
     *
     * # Defaults
     *
     * For BMS/BME, channels #11/13/15/19 and #21/23/25/29 use `WhiteKey`, #12/14/18 and #22/24/28
     * use `BlackKey`, #16 and #26 use `Scratch`, #17 and #27 use `FootPedal`.
     *
     * For PMS, channels #11/17/25 use `Button1`, #12/16/24 use `Button2`, #13/19/23 use `Button3`,
     * #14/18/22 use `Button4`, #15 uses `Button5`.
     */
    #[deriving(PartialEq,Eq)]
    pub enum KeyKind {
        /// White key, which mimics a real white key in the musical keyboard.
        WhiteKey,
        /// White key, but rendered yellow. This is used for simulating the O2Jam interface which
        /// has one yellow lane (mapped to spacebar) in middle of six other lanes (mapped to normal
        /// keys).
        WhiteKeyAlt,
        /// Black key, which mimics a real black key in the keyboard but rendered light blue as in
        /// Beatmania and other games.
        BlackKey,
        /// Scratch, rendered red. Scratch lane is wider than other "keys" and normally doesn't
        /// count as a key.
        Scratch,
        /// Foot pedal, rendered green. Otherwise has the same properties as scratch. The choice of
        /// color follows that of EZ2DJ, one of the first games that used this game element.
        FootPedal,
        /// White button. This and following "buttons" come from Pop'n Music, which has nine colored
        /// buttons. (White buttons constitute 1st and 9th of Pop'n Music buttons.) The "buttons"
        /// are wider than aforementioned "keys" but narrower than scratch and foot pedal.
        Button1,
        /// Yellow button (2nd and 8th of Pop'n Music buttons).
        Button2,
        /// Green button (3rd and 7th of Pop'n Music buttons).
        Button3,
        /// Navy button (4th and 6th of Pop'n Music buttons).
        Button4,
        /// Red button (5th of Pop'n Music buttons).
        Button5,
    }

    impl KeyKind {
        /// Returns a list of all supported key kinds.
        //
        // Rust: can this method be generated on the fly?
        pub fn all() -> &'static [KeyKind] {
            static ALL: [KeyKind, ..10] = [WhiteKey, WhiteKeyAlt, BlackKey, Scratch, FootPedal,
                                           Button1, Button2, Button3, Button4, Button5];
            ALL
        }

        /// Converts a mnemonic character to an appropriate key kind. Used for parsing a key
        /// specification (see also `KeySpec`).
        pub fn from_char(c: char) -> Option<KeyKind> {
            match c {
                'a' => Some(WhiteKey),
                'y' => Some(WhiteKeyAlt),
                'b' => Some(BlackKey),
                's' => Some(Scratch),
                'p' => Some(FootPedal),
                'q' => Some(Button1),
                'w' => Some(Button2),
                'e' => Some(Button3),
                'r' => Some(Button4),
                't' => Some(Button5),
                _   => None
            }
        }

        /// Converts an appropriate key kind to a mnemonic character. Used for environment variables
        /// (see also `read_keymap`).
        pub fn to_char(&self) -> char {
            match *self {
                WhiteKey    => 'a',
                WhiteKeyAlt => 'y',
                BlackKey    => 'b',
                Scratch     => 's',
                FootPedal   => 'p',
                Button1     => 'w',
                Button2     => 'e',
                Button3     => 'r',
                Button4     => 't',
                Button5     => 's'
            }
        }

        /**
         * Returns true if a kind counts as a "key". (C: `KEYKIND_IS_KEY`)
         *
         * This affects the number of keys displayed in the loading screen, and reflects a common
         * practice of counting "keys" in many games (e.g. Beatmania IIDX has 8 lanes including one
         * scratch but commonly said to have 7 "keys").
         */
        pub fn counts_as_key(&self) -> bool {
            *self != Scratch && *self != FootPedal
        }
    }

    //----------------------------------------------------------------------------------------------
    // object parameters

    /// Sound reference.
    #[deriving(PartialEq,Eq,Clone)]
    pub struct SoundRef(pub Key);

    impl Deref<Key> for SoundRef {
        fn deref<'a>(&'a self) -> &'a Key {
            let SoundRef(ref key) = *self;
            key
        }
    }

    /// Image reference.
    #[deriving(PartialEq,Eq,Clone)]
    pub struct ImageRef(pub Key);

    impl Deref<Key> for ImageRef {
        fn deref<'a>(&'a self) -> &'a Key {
            let ImageRef(ref key) = *self;
            key
        }
    }

    /// BGA layers. (C: `enum BGA_type`)
    #[deriving(PartialEq,Eq,Clone)]
    pub enum BGALayer {
        /// The lowest layer. BMS channel #04. (C: `BGA_LAYER`)
        Layer1 = 0,
        /// The middle layer. BMS channel #07. (C: `BGA2_LAYER`)
        Layer2 = 1,
        /// The highest layer. BMS channel #0A. (C: `BGA3_LAYER`)
        Layer3 = 2,
        /// The layer only displayed shortly after the MISS grade. It is technically not over
        /// `Layer3`, but several extensions to BMS assumes it. BMS channel #06.
        /// (C: `POORBGA_LAYER`)
        PoorBGA = 3
    }

    /// The number of BGA layers.
    pub const NLAYERS: uint = 4;

    /// Beats per minute. Used as a conversion factor between the time position and actual time
    /// in BMS.
    #[deriving(PartialEq,Clone)]
    pub struct BPM(pub f64);

    impl Deref<f64> for BPM {
        fn deref<'a>(&'a self) -> &'a f64 {
            let BPM(ref v) = *self;
            v
        }
    }

    impl BPM {
        /// Converts a measure to a millisecond. (C: `MEASURE_TO_MSEC`)
        pub fn measure_to_msec(&self, measure: f64) -> f64 { measure * 240000.0 / **self }

        /// Converts a millisecond to a measure. (C: `MSEC_TO_MEASURE`)
        pub fn msec_to_measure(&self, msec: f64) -> f64 { msec * **self / 240000.0 }
    }

    /// A duration from the particular point. It may be specified in measures or seconds. Used in
    /// the `Stop` object.
    #[deriving(PartialEq,Clone)]
    pub enum Duration { Seconds(f64), Measures(f64) }

    impl Duration {
        /// Calculates the actual milliseconds from the current BPM.
        pub fn to_msec(&self, bpm: BPM) -> f64 {
            match *self {
                Seconds(secs) => secs * 1000.0,
                Measures(measures) => bpm.measure_to_msec(measures)
            }
        }
    }

    /// A damage value upon the MISS grade. Normally it is specified in percents of the full gauge
    /// (as in `MAXGAUGE`), but sometimes it may cause an instant death. Used in the `Bomb` object
    /// (normal note objects have a fixed value).
    #[deriving(PartialEq,Clone)]
    pub enum Damage { GaugeDamage(f64), InstantDeath }

    //----------------------------------------------------------------------------------------------
    // object

    /// A data for objects (or object-like effects). Does not include the time information.
    #[deriving(PartialEq,Clone)]
    pub enum ObjData {
        /// Deleted object. Only used during various processing.
        Deleted,
        /// Visible object. Sound is played when the key is input inside the associated grading
        /// area. (C: `NOTE`)
        Visible(Lane, Option<SoundRef>),
        /// Invisible object. Sound is played when the key is input inside the associated grading
        /// area. No render nor grading performed. (C: `INVNOTE`)
        Invisible(Lane, Option<SoundRef>),
        /// Start of long note (LN). Sound is played when the key is down inside the associated
        /// grading area. (C: `LNSTART`)
        LNStart(Lane, Option<SoundRef>),
        /// End of LN. Sound is played when the start of LN is graded, the key was down and now up
        /// inside the associated grading area. (C: `LNDONE`)
        LNDone(Lane, Option<SoundRef>),
        /// Bomb. Pressing the key down at the moment that the object is on time causes
        /// the specified damage; sound is played in this case. No associated grading area.
        /// (C: `BOMB`)
        Bomb(Lane, Option<SoundRef>, Damage),
        /// Plays associated sound. (C: `BGM_CHANNEL`)
        BGM(SoundRef),
        /**
         * Sets the virtual BGA layer to given image. The layer itself may not be displayed
         * depending on the current game status. (C: `BGA_CHANNEL`)
         *
         * If the reference points to a movie, the movie starts playing; if the other layer had
         * the same movie started, it rewinds to the beginning. The resulting image from the movie
         * can be shared among multiple layers.
         */
        SetBGA(BGALayer, Option<ImageRef>),
        /// Sets the BPM. Negative BPM causes the chart scrolls backwards (and implicitly signals
        /// the end of the chart). (C: `BPM_CHANNEL`)
        SetBPM(BPM),
        /// Stops the scroll of the chart for given duration ("scroll stopper" hereafter).
        /// (C: `STOP_CHANNEL`)
        Stop(Duration)
    }

    /// Query operations for objects.
    pub trait ObjQueryOps {
        /// Returns true if the object is a visible object (`Visible`). (C: `obj->type == NOTE`)
        fn is_visible(&self) -> bool;
        /// Returns true if the object is an invisible object (`Invisible`).
        /// (C: `obj->type == INVNOTE`)
        fn is_invisible(&self) -> bool;
        /// Returns true if the object is a start of LN object (`LNStart`).
        /// (C: `obj->type == LNSTART`)
        fn is_lnstart(&self) -> bool;
        /// Returns true if the object is an end of LN object (`LNEnd`). (C: `obj->type == LNDONE`)
        fn is_lndone(&self) -> bool;
        /// Returns true if the object is either a start or an end of LN object.
        /// (C: `obj->type < NOTE`)
        fn is_ln(&self) -> bool;
        /// Returns true if the object is a bomb (`Bomb`). (C: `obj->type == BOMB`)
        fn is_bomb(&self) -> bool;
        /// Returns true if the object is soundable when it is the closest soundable object from
        /// the current position and the player pressed the key. Named "soundable" since it may
        /// choose not to play the associated sound. Note that not every object with sound is
        /// soundable. (C: `obj->type <= INVNOTE`)
        fn is_soundable(&self) -> bool;
        /// Returns true if the object is subject to grading. (C: `obj->type < INVNOTE`)
        fn is_gradable(&self) -> bool;
        /// Returns true if the object has a visible representation. (C: `obj->type != INVNOTE`)
        fn is_renderable(&self) -> bool;
        /// Returns true if the data is an object. (C: `IS_NOTE_CHANNEL(obj->chan)`)
        fn is_object(&self) -> bool;
        /// Returns true if the data is a BGM. (C: `obj->chan == BGM_CHANNEL`)
        fn is_bgm(&self) -> bool;
        /// Returns true if the data is a BGA. (C: `obj->chan == BGA_CHANNEL`)
        fn is_setbga(&self) -> bool;
        /// Returns true if the data is a BPM change. (C: `obj->chan == BPM_CHANNEL`)
        fn is_setbpm(&self) -> bool;
        /// Returns true if the data is a scroll stopper. (C: `obj->chan == STOP_CHANNEL`)
        fn is_stop(&self) -> bool;

        /// Returns an associated lane if the data is an object.
        fn object_lane(&self) -> Option<Lane>;
        /// Returns all sounds associated to the data.
        fn sounds(&self) -> Vec<SoundRef>;
        /// Returns all sounds played when key is pressed.
        fn keydown_sound(&self) -> Option<SoundRef>;
        /// Returns all sounds played when key is unpressed.
        fn keyup_sound(&self) -> Option<SoundRef>;
        /// Returns all sounds played when the object is activated while the corresponding key is
        /// currently pressed. Bombs are the only instance of this kind of sounds.
        fn through_sound(&self) -> Option<SoundRef>;
        /// Returns all images associated to the data.
        fn images(&self) -> Vec<ImageRef>;
        /// Returns an associated damage value when the object is activated.
        fn through_damage(&self) -> Option<Damage>;
    }

    /// Conversion operations for objects.
    pub trait ObjConvOps: ObjQueryOps {
        /// Returns a visible object with the same time, lane and sound as given object.
        fn to_visible(&self) -> Self;
        /// Returns an invisible object with the same time, lane and sound as given object.
        fn to_invisible(&self) -> Self;
        /// Returns a start of LN object with the same time, lane and sound as given object.
        fn to_lnstart(&self) -> Self;
        /// Returns an end of LN object with the same time, lane and sound as given object.
        fn to_lndone(&self) -> Self;
    }

    impl ObjQueryOps for ObjData {
        fn is_visible(&self) -> bool {
            match *self { Visible(..) => true, _ => false }
        }

        fn is_invisible(&self) -> bool {
            match *self { Invisible(..) => true, _ => false }
        }

        fn is_lnstart(&self) -> bool {
            match *self { LNStart(..) => true, _ => false }
        }

        fn is_lndone(&self) -> bool {
            match *self { LNDone(..) => true, _ => false }
        }

        fn is_ln(&self) -> bool {
            match *self { LNStart(..) | LNDone(..) => true, _ => false }
        }

        fn is_bomb(&self) -> bool {
            match *self { Bomb(..) => true, _ => false }
        }

        fn is_soundable(&self) -> bool {
            match *self { Visible(..) | Invisible(..) | LNStart(..) | LNDone(..) => true,
                         _ => false }
        }

        fn is_gradable(&self) -> bool {
            match *self { Visible(..) | LNStart(..) | LNDone(..) => true, _ => false }
        }

        fn is_renderable(&self) -> bool {
            match *self { Visible(..) | LNStart(..) | LNDone(..) | Bomb(..) => true, _ => false }
        }

        fn is_object(&self) -> bool {
            match *self { Visible(..) | Invisible(..) | LNStart(..) | LNDone(..) | Bomb(..) => true,
                         _ => false }
        }

        fn is_bgm(&self) -> bool {
            match *self { BGM(..) => true, _ => false }
        }

        fn is_setbga(&self) -> bool {
            match *self { SetBGA(..) => true, _ => false }
        }

        fn is_setbpm(&self) -> bool {
            match *self { SetBPM(..) => true, _ => false }
        }

        fn is_stop(&self) -> bool {
            match *self { Stop(..) => true, _ => false }
        }

        fn object_lane(&self) -> Option<Lane> {
            match *self {
                Visible(lane,_) | Invisible(lane,_) | LNStart(lane,_) |
                LNDone(lane,_) | Bomb(lane,_,_) => Some(lane),
                _ => None
            }
        }

        fn sounds(&self) -> Vec<SoundRef> {
            match *self {
                Visible(_,Some(sref)) => vec!(sref),
                Invisible(_,Some(sref)) => vec!(sref),
                LNStart(_,Some(sref)) => vec!(sref),
                LNDone(_,Some(sref)) => vec!(sref),
                Bomb(_,Some(sref),_) => vec!(sref),
                BGM(sref) => vec!(sref),
                _ => Vec::new()
            }
        }

        fn keydown_sound(&self) -> Option<SoundRef> {
            match *self { Visible(_,sref) | Invisible(_,sref) | LNStart(_,sref) => sref, _ => None }
        }

        fn keyup_sound(&self) -> Option<SoundRef> {
            match *self { LNDone(_,sref) => sref, _ => None }
        }

        fn through_sound(&self) -> Option<SoundRef> {
            match *self { Bomb(_,sref,_) => sref, _ => None }
        }

        fn images(&self) -> Vec<ImageRef> {
            match *self { SetBGA(_,Some(iref)) => vec!(iref), _ => Vec::new() }
        }

        fn through_damage(&self) -> Option<Damage> {
            match *self { Bomb(_,_,damage) => Some(damage), _ => None }
        }
    }

    impl ObjConvOps for ObjData {
        fn to_visible(&self) -> ObjData {
            match *self {
                Visible(lane,snd) | Invisible(lane,snd) |
                LNStart(lane,snd) | LNDone(lane,snd) => Visible(lane,snd),
                _ => panic!("to_visible for non-object")
            }
        }

        fn to_invisible(&self) -> ObjData {
            match *self {
                Visible(lane,snd) | Invisible(lane,snd) |
                LNStart(lane,snd) | LNDone(lane,snd) => Invisible(lane,snd),
                _ => panic!("to_invisible for non-object")
            }
        }

        fn to_lnstart(&self) -> ObjData {
            match *self {
                Visible(lane,snd) | Invisible(lane,snd) |
                LNStart(lane,snd) | LNDone(lane,snd) => LNStart(lane,snd),
                _ => panic!("to_lnstart for non-object")
            }
        }

        fn to_lndone(&self) -> ObjData {
            match *self {
                Visible(lane,snd) | Invisible(lane,snd) |
                LNStart(lane,snd) | LNDone(lane,snd) => LNDone(lane,snd),
                _ => panic!("to_lndone for non-object")
            }
        }
    }

    /// Game play data associated to the time axis. It contains both objects (which are also
    /// associated to lanes) and object-like effects.
    #[deriving(PartialEq,Clone)]
    pub struct Obj {
        /// Time position in measures.
        pub time: f64,
        /// Actual data.
        pub data: ObjData
    }

    #[allow(non_snake_case)]
    impl Obj {
        /// Creates a `Visible` object.
        pub fn Visible(time: f64, lane: Lane, sref: Option<Key>) -> Obj {
            Obj { time: time, data: Visible(lane, sref.map(SoundRef)) }
        }

        /// Creates an `Invisible` object.
        pub fn Invisible(time: f64, lane: Lane, sref: Option<Key>) -> Obj {
            Obj { time: time, data: Invisible(lane, sref.map(SoundRef)) }
        }

        /// Creates an `LNStart` object.
        pub fn LNStart(time: f64, lane: Lane, sref: Option<Key>) -> Obj {
            Obj { time: time, data: LNStart(lane, sref.map(SoundRef)) }
        }

        /// Creates an `LNDone` object.
        pub fn LNDone(time: f64, lane: Lane, sref: Option<Key>) -> Obj {
            Obj { time: time, data: LNDone(lane, sref.map(SoundRef)) }
        }

        /// Creates a `Bomb` object.
        pub fn Bomb(time: f64, lane: Lane, sref: Option<Key>, damage: Damage) -> Obj {
            Obj { time: time, data: Bomb(lane, sref.map(SoundRef), damage) }
        }

        /// Creates a `BGM` object.
        pub fn BGM(time: f64, sref: Key) -> Obj {
            Obj { time: time, data: BGM(SoundRef(sref)) }
        }

        /// Creates a `SetBGA` object.
        pub fn SetBGA(time: f64, layer: BGALayer, iref: Option<Key>) -> Obj {
            Obj { time: time, data: SetBGA(layer, iref.map(ImageRef)) }
        }

        /// Creates a `SetBPM` object.
        pub fn SetBPM(time: f64, bpm: BPM) -> Obj {
            Obj { time: time, data: SetBPM(bpm) }
        }

        /// Creates a `Stop` object.
        pub fn Stop(time: f64, duration: Duration) -> Obj {
            Obj { time: time, data: Stop(duration) }
        }

        /// Returns the number of a measure containing this object.
        pub fn measure(&self) -> int { self.time.floor() as int }
    }

    impl ObjQueryOps for Obj {
        fn is_visible(&self) -> bool { self.data.is_visible() }
        fn is_invisible(&self) -> bool { self.data.is_invisible() }
        fn is_lnstart(&self) -> bool { self.data.is_lnstart() }
        fn is_lndone(&self) -> bool { self.data.is_lndone() }
        fn is_ln(&self) -> bool { self.data.is_ln() }
        fn is_bomb(&self) -> bool { self.data.is_bomb() }
        fn is_soundable(&self) -> bool { self.data.is_soundable() }
        fn is_gradable(&self) -> bool { self.data.is_gradable() }
        fn is_renderable(&self) -> bool { self.data.is_renderable() }
        fn is_object(&self) -> bool { self.data.is_object() }
        fn is_bgm(&self) -> bool { self.data.is_bgm() }
        fn is_setbga(&self) -> bool { self.data.is_setbga() }
        fn is_setbpm(&self) -> bool { self.data.is_setbpm() }
        fn is_stop(&self) -> bool { self.data.is_stop() }

        fn object_lane(&self) -> Option<Lane> { self.data.object_lane() }
        fn sounds(&self) -> Vec<SoundRef> { self.data.sounds() }
        fn keydown_sound(&self) -> Option<SoundRef> { self.data.keydown_sound() }
        fn keyup_sound(&self) -> Option<SoundRef> { self.data.keyup_sound() }
        fn through_sound(&self) -> Option<SoundRef> { self.data.through_sound() }
        fn images(&self) -> Vec<ImageRef> { self.data.images() }
        fn through_damage(&self) -> Option<Damage> { self.data.through_damage() }
    }

    impl ObjConvOps for Obj {
        fn to_visible(&self) -> Obj { Obj { time: self.time, data: self.data.to_visible() } }
        fn to_invisible(&self) -> Obj { Obj { time: self.time, data: self.data.to_invisible() } }
        fn to_lnstart(&self) -> Obj { Obj { time: self.time, data: self.data.to_lnstart() } }
        fn to_lndone(&self) -> Obj { Obj { time: self.time, data: self.data.to_lndone() } }
    }

    //----------------------------------------------------------------------------------------------
    // BMS data

    /// Default BPM. This value comes from the original BMS specification.
    pub const DEFAULT_BPM: BPM = BPM(130.0);

    /**
     * Blit commands, which manipulate the image after the image had been loaded. This maps to BMS
     * #BGA command. (C: `struct blitcmd`)
     *
     * Blitting occurs from the region `(x1,y1)-(x2,y2)` in the source surface to the region
     * `(dx,dy)-(dx+(x2-x1),dy+(y2-y1))` in the destination surface. The rectangular region contains
     * the upper-left corner but not the lower-right corner. The region is clipped to make
     * the upper-left corner has non-negative coordinates and the size of the region doesn't exceed
     * 256 by 256 pixels.
     */
    pub struct BlitCmd {
        pub dst: ImageRef, pub src: ImageRef,
        pub x1: int, pub y1: int, pub x2: int, pub y2: int, pub dx: int, pub dy: int
    }

    /// A value of BMS #PLAYER command signifying Single Play (SP), where only channels #1x are used
    /// for the game play.
    pub const SINGLE_PLAY: int = 1;
    /// A value of BMS #PLAYER command signifying Couple Play, where channels #1x and #2x renders to
    /// the different panels. They are originally meant to be played by different players with
    /// separate gauges and scores, but this mode of game play is increasingly unsupported by modern
    /// implementations. Angolmois has only a limited support for Couple Play.
    pub const COUPLE_PLAY: int = 2;
    /// A value of BMS #PLAYER command signifying Double Play (DP), where both channels #1x and #2x
    /// renders to a single wide panel. The chart is still meant to be played by one person.
    pub const DOUBLE_PLAY: int = 3;

    /// Loaded BMS data. It is not a global state unlike C.
    pub struct Bms {
        /// Title. Maps to BMS #TITLE command. (C: `string[S_TITLE]`)
        pub title: Option<String>,
        /// Genre. Maps to BMS #GENRE command. (C: `string[S_GENRE]`)
        pub genre: Option<String>,
        /// Artist. Maps to BMS #ARTIST command. (C: `string[S_ARTIST]`)
        pub artist: Option<String>,
        /// Path to an image for loading screen. Maps to BMS #STAGEFILE command.
        /// (C: `string[S_STAGEFILE]`)
        pub stagefile: Option<String>,
        /// A base path used for loading all other resources. Maps to BMS #PATH_WAV command.
        /// (C: `string[S_BASEPATH]`)
        pub basepath: Option<String>,

        /// Game mode. One of `SINGLE_PLAY`(1), `COUPLE_PLAY`(2) or `DOUBLE_PLAY`(3). Maps to BMS
        /// #PLAYER command. (C: `value[V_PLAYER]`)
        pub player: int,
        /// Game level. Does not affect the actual game play. Maps to BMS #PLAYLEVEL command.
        /// (C: `value[V_PLAYLEVEL]`)
        pub playlevel: int,
        /// Gauge difficulty. Higher is easier. Maps to BMS #RANK command. (C: `value[V_RANK]`)
        pub rank: int,

        /// Initial BPM. (C: `initbpm`)
        pub initbpm: BPM,
        /// Paths to sound file relative to `basepath` or BMS file. (C: `sndpath`)
        pub sndpath: Vec<Option<String>>,
        /// Paths to image/movie file relative to `basepath` or BMS file. (C: `imgpath`)
        pub imgpath: Vec<Option<String>>,
        /// List of blit commands to be executed after `imgpath` is loaded. (C: `blitcmd`)
        pub blitcmd: Vec<BlitCmd>,

        /// List of objects sorted by the position. (C: `objs`)
        pub objs: Vec<Obj>,
        /// The scaling factor of measures. Defaults to 1.0. (C: `shortens`)
        pub shortens: Vec<f64>,
        /// The number of measures after the origin, i.e. the length of the BMS file. The play stops
        /// after the last measure. (C: `length`)
        pub nmeasures: uint
    }

    impl Bms {
        /// Creates a default value of BMS data.
        pub fn new() -> Bms {
            Bms { title: None, genre: None, artist: None, stagefile: None, basepath: None,
                  player: SINGLE_PLAY, playlevel: 0, rank: 2, initbpm: DEFAULT_BPM,
                  sndpath: Vec::from_elem(MAXKEY as uint, None),
                  imgpath: Vec::from_elem(MAXKEY as uint, None), blitcmd: Vec::new(),
                  objs: Vec::new(), shortens: Vec::new(), nmeasures: 0 }
        }

        /// Returns a scaling factor of given measure number. The default scaling factor is 1.0, and
        /// that value applies to any out-of-bound measures. (C: `shorten`)
        pub fn shorten(&self, measure: int) -> f64 {
            if measure < 0 || measure as uint >= self.shortens.len() {
                1.0
            } else {
                self.shortens[measure as uint]
            }
        }

        /// Calculates the virtual time that is `offset` measures away from the virtual time `base`.
        /// This takes account of the scaling factor, so if first four measures are scaled by 1/4,
        /// then `adjust_object_time(0.0, 2.0)` results in `5.0`. (C: `adjust_object_time`)
        pub fn adjust_object_time(&self, base: f64, offset: f64) -> f64 {
            let basemeasure = base.floor() as int;
            let baseshorten = self.shorten(basemeasure);
            let basefrac = base - basemeasure as f64;
            let tonextmeasure = (1.0 - basefrac) * baseshorten;
            if offset < tonextmeasure {
                base + offset / baseshorten
            } else {
                let mut offset = offset - tonextmeasure;
                let mut i = basemeasure + 1;
                let mut curshorten = self.shorten(i);
                while offset >= curshorten {
                    offset -= curshorten;
                    i += 1;
                    curshorten = self.shorten(i);
                }
                i as f64 + offset / curshorten
            }
        }

        /// Calculates an adjusted offset between the virtual time `base` and `base + offset`.
        /// This takes account of the measure scaling factor, so for example, the adjusted offset
        /// between the virtual time 0.0 and 2.0 is, if the measure #000 is scaled by 1.2x,
        /// 2.2 measures instead of 2.0 measures. (C: `adjust_object_position`)
        pub fn adjust_object_position(&self, base: f64, time: f64) -> f64 {
            let basemeasure = base.floor() as int;
            let timemeasure = time.floor() as int;
            let basefrac = base - basemeasure as f64;
            let timefrac = time - timemeasure as f64;
            let mut pos = timefrac * self.shorten(timemeasure) -
                          basefrac * self.shorten(basemeasure);
            for i in range(basemeasure, timemeasure) {
                pos += self.shorten(i);
            }
            pos
        }
    }

    //----------------------------------------------------------------------------------------------
    // parsing

    /// Converts a single alphanumeric (base-36) letter to an integer. (C: `getdigit`)
    fn getdigit(n: char) -> Option<int> {
        match n {
            '0'...'9' => Some((n as int) - ('0' as int)),
            'a'...'z' => Some((n as int) - ('a' as int) + 10),
            'A'...'Z' => Some((n as int) - ('A' as int) + 10),
            _ => None
        }
    }

    /// Converts the first two letters of `s` to a `Key`. (C: `key2index`)
    pub fn key2index(s: &[char]) -> Option<int> {
        if s.len() < 2 { return None; }
        getdigit(s[0]).and_then(|a| {
            getdigit(s[1]).map(|b| { a * 36 + b })
        })
    }

    impl FromStrPrefix for Key {
        fn from_str_prefix<'a>(s: &'a str) -> Option<(Key, &'a str)> {
            if s.len() < 2 { return None; }
            let str::CharRange {ch:c1, next:p1} = s.char_range_at(0);
            getdigit(c1).and_then(|a| {
                let str::CharRange {ch:c2, next:p2} = s.char_range_at(p1);
                getdigit(c2).map(|b| {
                    assert!(p2 == 2); // both characters should be in ASCII
                    (Key(a * 36 + b), s[p2..])
                })
            })
        }
    }

    /// A wrapper type for a measure number. Only used for parsing.
    struct Measure(uint);

    impl FromStrPrefix for Measure {
        fn from_str_prefix<'a>(s: &'a str) -> Option<(Measure, &'a str)> {
            let isdigit = |c| '0' <= c && c <= '9';
            if s.len() >= 3 && isdigit(s.char_at(0)) && isdigit(s.char_at(1))
                            && isdigit(s.char_at(2)) {
                let measure = from_str::<uint>(s[..3]).unwrap();
                Some((Measure(measure), s[3..]))
            } else {
                None
            }
        }
    }

    /// Reads and parses the BMS file with given RNG from given reader.
    pub fn parse_bms_from_reader<R:Rng>(f: &mut Reader, r: &mut R) -> io::IoResult<Bms> {
        /// The list of recognized prefixes of directives. The longest prefix should come first.
        /// Also note that not all recognized prefixes are processed (counterexample being `ENDSW`).
        /// (C: `bmsheader`)
        static BMS_HEADER: &'static [&'static str] = &[
            "TITLE", "GENRE", "ARTIST", "STAGEFILE", "PATH_WAV", "BPM",
            "PLAYER", "PLAYLEVEL", "RANK", "LNTYPE", "LNOBJ", "WAV", "BMP",
            "BGA", "STOP", "STP", "RANDOM", "SETRANDOM", "ENDRANDOM", "IF",
            "ELSEIF", "ELSE", "ENDSW", "END"];

        let mut bms = Bms::new();

        /// The state of the block, for determining which lines should be processed.
        #[deriving(PartialEq)]
        enum BlockState {
            /// Not contained in the #IF block. (C: `state == -1`)
            Outside,
            /// Active. (C: `state == 0`)
            Process,
            /// Inactive, but (for the purpose of #IF/#ELSEIF/#ELSE/#ENDIF structure) can move to
            /// `Process` state when matching clause appears. (C: `state == 1`)
            Ignore,
            /// Inactive and won't be processed until the end of block. (C: `state == 2`)
            NoFurther
        }

        impl BlockState {
            /// Returns true if lines should be ignored in the current block given that the parent
            /// block was active. (C: `state > 0`)
            fn inactive(&self) -> bool {
                match *self { Outside | Process => false, Ignore | NoFurther => true }
            }
        }

        /**
         * Block information. The parser keeps a list of nested blocks and determines if
         * a particular line should be processed or not. (C: `struct rnd`)
         *
         * Angomlois actually recognizes only one kind of blocks, starting with #RANDOM or
         * #SETRANDOM and ending with #ENDRANDOM or #END(IF) outside an #IF block. An #IF block is
         * a state within #RANDOM, so it follows that #RANDOM/#SETRANDOM blocks can nest but #IF
         * can't nest unless its direct parent is #RANDOM/#SETRANDOM.
         */
        #[deriving(PartialEq)]
        struct Block {
            /// A generated value if any. It can be `None` if this block is the topmost one (which
            /// is actually not a block but rather a sentinel) or the last `#RANDOM` or `#SETRANDOM`
            /// command was invalid, and #IF in that case will always evaluates to false. (C: `val`
            /// field)
            val: Option<int>,
            /// The state of the block. (C: `state` field)
            state: BlockState,
            /// True if the parent block is already ignored so that this block should be ignored
            /// no matter what `state` is. (C: `skip` field)
            skip: bool
        }

        impl Block {
            /// Returns true if lines should be ignored in the current block.
            fn inactive(&self) -> bool { self.skip || self.state.inactive() }
        }

        // A list of nested blocks. (C: `rnd`)
        let mut blk = vec!(Block { val: None, state: Outside, skip: false });

        /// An unprocessed data line of BMS file.
        #[deriving(Clone)]
        struct BmsLine { measure: uint, chan: Key, data: String }

        // A list of unprocessed data lines. They have to be sorted with a stable algorithm and
        // processed in the order of measure number. (C: `bmsline`)
        let mut bmsline = Vec::new();
        // A table of BPMs. Maps to BMS #BPMxx command. (C: `bpmtab`)
        let mut bpmtab = Vec::from_elem(MAXKEY as uint, DEFAULT_BPM);
        // A table of the length of scroll stoppers. Maps to BMS #STOP/#STP commands. (C: `stoptab`)
        let mut stoptab = Vec::from_elem(MAXKEY as uint, Seconds(0.0));

        // Allows LNs to be specified as a consecutive row of same or non-00 alphanumeric keys (MGQ
        // type, #LNTYPE 2). The default is to specify LNs as two endpoints (RDM type, #LNTYPE 1).
        // (C: `value[V_LNTYPE]`)
        let mut consecutiveln = false;

        // An end-of-LN marker used in LN specification for channels #1x/2x. Maps to BMS #LNOBJ
        // command. (C: `value[V_LNOBJ]`)
        let mut lnobj = None;

        let file = try!(f.read_to_end());
        for line0 in file[].split(|&ch| ch == 10u8) {
            let line0 = String::from_utf8_lossy(line0).into_string();
            let line = line0[];

            // skip non-command lines
            let line = line.trim_left();
            if !line.starts_with("#") { continue; }
            let line = line[1..];

            // search for header prefix. the header list (`BMS_HEADER`) is in the decreasing order
            // of prefix length.
            let mut prefix = "";
            for &header in BMS_HEADER.iter() {
                use std::ascii::AsciiExt;
                if line.len() >= header.len() && line[..header.len()].to_ascii_upper()[] == header {
                    prefix = header;
                    break;
                }
            }
            let line = line[prefix.len()..];

            // Common readers.
            macro_rules! read(
                (string $string:ident) => ({
                    let mut text = "";
                    if lex!(line; ws, str* -> text, ws*, !) {
                        bms.$string = Some(text.to_string());
                    }
                });
                (value $value:ident) => ({
                    lex!(line; ws, int -> bms.$value);
                });
                (path $paths:ident) => ({
                    let mut key = Key(-1);
                    let mut path = "";
                    if lex!(line; Key -> key, ws, str -> path, ws*, !) {
                        let Key(key) = key;
                        bms.$paths[mut][key as uint] = Some(path.to_string());
                    }
                })
            )

            assert!(!blk.is_empty());
            match (prefix, blk.last().unwrap().inactive()) {
                // #TITLE|#GENRE|#ARTIST|#STAGEFILE|#PATH_WAV <string>
                ("TITLE", false) => read!(string title),
                ("GENRE", false) => read!(string genre),
                ("ARTIST", false) => read!(string artist),
                ("STAGEFILE", false) => read!(string stagefile),
                ("PATH_WAV", false) => read!(string basepath),

                // #BPM <float> or #BPMxx <float>
                ("BPM", false) => {
                    let mut key = Key(-1);
                    let mut bpm = 0.0;
                    if lex!(line; Key -> key, ws, f64 -> bpm) {
                        let Key(key) = key;
                        bpmtab[mut][key as uint] = BPM(bpm);
                    } else if lex!(line; ws, f64 -> bpm) {
                        bms.initbpm = BPM(bpm);
                    }
                }

                // #PLAYER|#PLAYLEVEL|#RANK <int>
                ("PLAYER", false) => read!(value player),
                ("PLAYLEVEL", false) => read!(value playlevel),
                ("RANK", false) => read!(value rank),

                // #LNTYPE <int>
                ("LNTYPE", false) => {
                    let mut lntype = 1;
                    if lex!(line; ws, int -> lntype) {
                        consecutiveln = lntype == 2;
                    }
                }
                // #LNOBJ <key>
                ("LNOBJ", false) => {
                    let mut key = Key(-1);
                    if lex!(line; ws, Key -> key) { lnobj = Some(key); }
                }

                // #WAVxx|#BMPxx <path>
                ("WAV", false) => read!(path sndpath),
                ("BMP", false) => read!(path imgpath),

                // #BGAxx yy <int> <int> <int> <int> <int> <int>
                ("BGA", false) => {
                    let mut dst = Key(0);
                    let mut src = Key(0);
                    let mut bc = BlitCmd { dst: ImageRef(Key(0)), src: ImageRef(Key(0)),
                                           x1: 0, y1: 0, x2: 0, y2: 0, dx: 0, dy: 0 };
                    if lex!(line; Key -> dst, ws, Key -> src, ws,
                                  int -> bc.x1, ws, int -> bc.y1, ws,
                                  int -> bc.x2, ws, int -> bc.y2, ws,
                                  int -> bc.dx, ws, int -> bc.dy) {
                        bc.src = ImageRef(src);
                        bc.dst = ImageRef(dst);
                        bms.blitcmd.push(bc);
                    }
                }

                // #STOPxx <int>
                ("STOP", false) => {
                    let mut key = Key(-1);
                    let mut duration = 0;
                    if lex!(line; Key -> key, ws, int -> duration) {
                        let Key(key) = key;
                        stoptab[mut][key as uint] = Measures(duration as f64 / 192.0);
                    }
                }

                // #STP<int>.<int> <int>
                ("STP", false) => {
                    let mut measure = Measure(0);
                    let mut frac = 0;
                    let mut duration = 0;
                    if lex!(line; Measure -> measure, lit '.', uint -> frac, ws,
                                  int -> duration) && duration > 0 {
                        let Measure(measure) = measure;
                        let pos = measure as f64 + frac as f64 * 0.001;
                        let dur = Seconds(duration as f64 * 0.001);
                        bms.objs.push(Obj::Stop(pos, dur));
                    }
                }

                // #RANDOM|#SETRANDOM <int>
                ("RANDOM", _) |
                ("SETRANDOM", _) => {
                    let mut val = 0;
                    if lex!(line; ws, int -> val) {
                        let val = if val <= 0 {None} else {Some(val)};

                        // do not generate a random value if the entire block is skipped (but it
                        // still marks the start of block)
                        let inactive = blk.last().unwrap().inactive();
                        let generated = val.and_then(|val| {
                            if prefix == "SETRANDOM" {
                                Some(val)
                            } else if !inactive {
                                Some(r.gen_range(1, val + 1))
                            } else {
                                None
                            }
                        });
                        blk.push(Block { val: generated, state: Outside, skip: inactive });
                    }
                }

                // #ENDRANDOM
                ("ENDRANDOM", _) => {
                    if blk.len() > 1 { blk.pop(); }
                }

                // #IF|#ELSEIF <int>
                ("IF", _) |
                ("ELSEIF", _) => {
                    let mut val = 0;
                    if lex!(line; ws, int -> val) {
                        let val = if val <= 0 {None} else {Some(val)};

                        let last = blk.last_mut().unwrap();
                        last.state =
                            if (prefix == "IF" && !last.state.inactive()) || last.state == Ignore {
                                if val.is_none() || val != last.val {Ignore} else {Process}
                            } else {
                                NoFurther
                            };
                    }
                }

                // #ELSE
                ("ELSE", _) => {
                    let last = blk.last_mut().unwrap();
                    last.state = if last.state == Ignore {Process} else {NoFurther};
                }

                // #END(IF)
                ("END", _) => {
                    for &idx in blk.iter().rposition(|&i| i.state != Outside).iter() {
                        if idx > 0 { blk.truncate(idx + 1); }
                    }

                    blk.last_mut().unwrap().state = Outside;
                }

                // #nnnmm:...
                ("", false) => {
                    let mut measure = Measure(0);
                    let mut chan = Key(0);
                    let mut data = "";
                    if lex!(line; Measure -> measure, Key -> chan, lit ':', ws*,
                                  str -> data, ws*, !) {
                        let Measure(measure) = measure;
                        bmsline.push(BmsLine { measure: measure, chan: chan,
                                               data: data.to_string() })
                    }
                }

                (_, _) => {}
            }
        }

        // Poor BGA defined by #BMP00 wouldn't be played if it is a movie. We can't just let it
        // played at the beginning of the chart as the "beginning" is not always 0.0 (actually,
        // `originoffset`). Thus we add an artificial BGA object at time 0.0 only when the other
        // poor BGA does not exist at this position. (C: `poorbgafix`)
        let mut poorbgafix = true;

        // Indices to last visible object per channels. A marker specified by #LNOBJ will turn
        // this last object to the start of LN. (C: `prev12`)
        let mut lastvis: [Option<uint>, ..NLANES] = [None, ..NLANES];

        // Indices to last LN start or end inserted (and not finalized yet) per channels.
        // If `consecutiveln` is on (#LNTYPE 2), the position of referenced object gets updated
        // during parsing; if off (#LNTYPE 1), it is solely used for checking if we are inside
        // the LN or not. (C: `prev56`)
        let mut lastln: [Option<uint>, ..NLANES] = [None, ..NLANES];

        {
            // Adds an object. Objects are sorted by its position later.
            let add = |bms: &mut Bms, obj: Obj| { bms.objs.push(obj); };

            // Adds an object and returns its position. LN parsing generally mutates the existing
            // object for simplicity.
            let mark = |bms: &mut Bms, obj: Obj| -> Option<uint> {
                let marked = bms.objs.len();
                bms.objs.push(obj);
                Some(marked)
            };

            // Handles a non-00 alphanumeric key `v` positioned at the particular channel `chan` and
            // particular position `t`. The position `t2` next to `t` is used for some cases that
            // an alphanumeric key designates an area rather than a point.
            let handle_key = |bms: &mut Bms, chan: Key, t: f64, t2: f64, v: Key| {
                match *chan {
                    // channel #01: BGM
                    1 => { add(bms, Obj::BGM(t, v)); }

                    // channel #03: BPM as an hexadecimal key
                    3 => {
                        for &v in v.to_hex().iter() {
                            add(bms, Obj::SetBPM(t, BPM(v as f64)))
                        }
                    }

                    // channel #04: BGA layer 1
                    4 => { add(bms, Obj::SetBGA(t, Layer1, Some(v))); }

                    // channel #06: POOR BGA
                    6 => {
                        add(bms, Obj::SetBGA(t, PoorBGA, Some(v)));
                        poorbgafix = false; // we don't add artificial BGA
                    }

                    // channel #07: BGA layer 2
                    7 => { add(bms, Obj::SetBGA(t, Layer2, Some(v))); }

                    // channel #08: BPM defined by #BPMxx
                    // TODO bpmtab validity check
                    8 => { add(bms, Obj::SetBPM(t, bpmtab[*v as uint])); }

                    // channel #09: scroll stopper defined by #STOPxx
                    // TODO stoptab validity check
                    9 => { add(bms, Obj::Stop(t, stoptab[*v as uint])); }

                    // channel #0A: BGA layer 3
                    10 => { add(bms, Obj::SetBGA(t, Layer3, Some(v))); }

                    // channels #1x/2x: visible object, possibly LNs when #LNOBJ is in active
                    36/*1*36*/...107/*3*36-1*/ => {
                        let lane = Lane::from_channel(chan);
                        if lnobj.is_some() && lnobj == Some(v) {
                            // change the last inserted visible object to the start of LN if any.
                            let lastvispos = lastvis[*lane];
                            for &pos in lastvispos.iter() {
                                assert!(bms.objs[pos].is_visible());
                                bms.objs[mut][pos] = bms.objs[pos].to_lnstart();
                                add(bms, Obj::LNDone(t, lane, Some(v)));
                                lastvis[*lane] = None;
                            }
                        } else {
                            lastvis[*lane] = mark(bms, Obj::Visible(t, lane, Some(v)));
                        }
                    }

                    // channels #3x/4x: invisible object
                    108/*3*36*/...179/*5*36-1*/ => {
                        let lane = Lane::from_channel(chan);
                        add(bms, Obj::Invisible(t, lane, Some(v)));
                    }

                    // channels #5x/6x, #LNTYPE 1: LN endpoints
                    180/*5*36*/...251/*7*36-1*/ if !consecutiveln => {
                        let lane = Lane::from_channel(chan);

                        // a pair of non-00 alphanumeric keys designate one LN. if there are an odd
                        // number of them, the last LN is implicitly closed later.
                        if lastln[*lane].is_some() {
                            lastln[*lane] = None;
                            add(bms, Obj::LNDone(t, lane, Some(v)));
                        } else {
                            lastln[*lane] = mark(bms, Obj::LNStart(t, lane, Some(v)));
                        }
                    }

                    // channels #5x/6x, #LNTYPE 2: LN areas
                    180/*5*36*/...251/*7*36-1*/ if consecutiveln => {
                        let lane = Lane::from_channel(chan);

                        // one non-00 alphanumeric key, in the absence of other information,
                        // inserts one complete LN starting at `t` and ending at `t2`.
                        //
                        // the next non-00 alphanumeric key also inserts one complete LN
                        // from `t` to `t2`, unless there is already an end of LN at `t`
                        // in which case the end of LN is simply moved from `t` to `t2`
                        // (effectively increasing the length of previous LN).
                        match lastln[*lane] {
                            Some(pos) if bms.objs[pos].time == t => {
                                assert!(bms.objs[pos].is_lndone());
                                bms.objs[mut][pos].time = t2;
                            }
                            _ => {
                                add(bms, Obj::LNStart(t, lane, Some(v)));
                                lastln[mut][*lane] = mark(bms, Obj::LNDone(t2, lane, Some(v)));
                            }
                        }
                    }

                    // channels #Dx/Ex: bombs, base-36 damage value (unit of 0.5% of the full gauge)
                    // or instant death (ZZ)
                    468/*0xD*36*/...539/*0xF*36-1*/ => {
                        let lane = Lane::from_channel(chan);
                        let damage = match v {
                            Key(v @ 1...200) => Some(GaugeDamage(v as f64 / 200.0)),
                            Key(1295) => Some(InstantDeath), // XXX 1295=MAXKEY-1
                            _ => None
                        };
                        for &damage in damage.iter() {
                            add(bms, Obj::Bomb(t, lane, Some(Key(0)), damage));
                        }
                    }

                    // unsupported: channels #0B/0C/0D/0E (BGA opacity), #97/98 (sound volume),
                    // #99 (text), #A0 (dynamic #RANK), #A1/A2/A3/A4 (BGA color key update),
                    // #A5 (BGA on keypress), #A6 (player-specific option)
                    _ => {}
                }
            };

            // loops over the sorted bmslines
            bmsline.sort_by(|a, b| (a.measure, b.chan).cmp(&(a.measure, b.chan)));
            for line in bmsline.iter() {
                if line.chan == Key(2) {
                    let mut shorten = 0.0;
                    if lex!(line.data[]; ws*, f64 -> shorten) {
                        if shorten > 0.001 {
                            if bms.shortens.len() <= line.measure {
                                let ncopies = line.measure - bms.shortens.len() + 1;
                                bms.shortens.grow(ncopies, 1.0);
                            }
                            bms.shortens[mut][line.measure] = shorten;
                        }
                    }
                } else {
                    let measure = line.measure as f64;
                    let data: Vec<char> = line.data[].chars().collect();
                    let max = data.len() / 2 * 2;
                    let count = max as f64;
                    for i in iter::range_step(0, max, 2) {
                        let v = key2index(data[i..i+2]);
                        for &v in v.iter() {
                            if v != 0 { // ignores 00
                                let t = measure + i as f64 / count;
                                let t2 = measure + (i + 2) as f64 / count;
                                handle_key(&mut bms, line.chan, t, t2, Key(v));
                            }
                        }
                    }
                }
            }
        }

        if poorbgafix {
            bms.objs.push(Obj::SetBGA(0.0, PoorBGA, Some(Key(0))));
        }

        // fix the unterminated longnote
        bms.nmeasures = bmsline.last().map_or(0, |l| l.measure) + 1;
        let endt = bms.nmeasures as f64;
        for i in range(0, NLANES) {
            if lastvis[i].is_some() || (!consecutiveln && lastln[i].is_some()) {
                bms.objs.push(Obj::LNDone(endt, Lane(i), None));
            }
        }

        Ok(bms)
    }

    /// Reads and parses the BMS file with given RNG. (C: `parse_bms`)
    pub fn parse_bms<R:Rng>(bmspath: &str, r: &mut R) -> io::IoResult<Bms> {
        let mut f = try!(io::File::open(&Path::new(bmspath)));
        parse_bms_from_reader(&mut f, r)
    }

    //----------------------------------------------------------------------------------------------
    // key specification

    /// The key specification. Specifies the order and apperance of lanes. Once determined from
    /// the options and BMS file, the key specification is fixed and independent of other data
    /// (e.g. `#PLAYER` value).
    pub struct KeySpec {
        /// The number of lanes on the left side. This number is significant only when Couple Play
        /// is used. (C: `nleftkeys`)
        pub split: uint,
        /// The order of significant lanes. The first `nleftkeys` lanes go to the left side and
        /// the remaining lanes (C: `nrightkeys`) go to the right side. (C: `keyorder`)
        pub order: Vec<Lane>,
        /// The type of lanes. (C: `keykind`)
        pub kinds: Vec<Option<KeyKind>>
    }

    impl KeySpec {
        /// Returns a number of lanes that count towards "keys". Notably scratches and pedals do not
        /// count as keys. (C: `nkeys`)
        pub fn nkeys(&self) -> uint {
            let mut nkeys = 0;
            for kind in self.kinds.iter().filter_map(|kind| *kind) {
                if kind.counts_as_key() { nkeys += 1; }
            }
            nkeys
        }

        /// Returns a list of lanes on the left side, from left to right.
        pub fn left_lanes<'r>(&'r self) -> &'r [Lane] {
            assert!(self.split <= self.order.len());
            self.order[..self.split]
        }

        /// Returns a list of lanes on the right side if any, from left to right.
        pub fn right_lanes<'r>(&'r self) -> &'r [Lane] {
            assert!(self.split <= self.order.len());
            self.order[self.split..]
        }
    }

    /// Parses the key specification from the string. (C: `parse_key_spec`)
    pub fn parse_key_spec(s: &str) -> Option<Vec<(Lane, KeyKind)>> {
        let mut specs = Vec::new();
        let mut s = s.trim_left();
        while !s.is_empty() {
            let mut chan = Key(0);
            let mut kind = '\x00';
            if !lex!(s; Key -> chan, char -> kind, ws*, str* -> s, !) {
                return None;
            }
            match (chan, KeyKind::from_char(kind)) {
                (Key(chan @ 36/*1*36*/...107/*3*36-1*/), Some(kind)) => {
                    specs.push((Lane(chan as uint - 1*36), kind));
                }
                (_, _) => { return None; }
            }
        }
        Some(specs)
    }

    /// A list of well-known key specifications. (C: `presets`)
    static PRESETS: &'static [(&'static str, &'static str, &'static str)] = &[
        // 5-key BMS, SP/DP
        ("5",     "16s 11a 12b 13a 14b 15a", ""),
        ("10",    "16s 11a 12b 13a 14b 15a", "21a 22b 23a 24b 25a 26s"),
        // 5-key BMS with a foot pedal, SP/DP
        ("5/fp",  "16s 11a 12b 13a 14b 15a 17p", ""),
        ("10/fp", "16s 11a 12b 13a 14b 15a 17p", "27p 21a 22b 23a 24b 25a 26s"),
        // 7-key BME, SP/DP
        ("7",     "16s 11a 12b 13a 14b 15a 18b 19a", ""),
        ("14",    "16s 11a 12b 13a 14b 15a 18b 19a", "21a 22b 23a 24b 25a 28b 29a 26s"),
        // 7-key BME with a foot pedal, SP/DP
        ("7/fp",  "16s 11a 12b 13a 14b 15a 18b 19a 17p", ""),
        ("14/fp", "16s 11a 12b 13a 14b 15a 18b 19a 17p", "27p 21a 22b 23a 24b 25a 28b 29a 26s"),
        // 9-key PMS (#PLAYER 3)
        ("9",     "11q 12w 13e 14r 15t 22r 23e 24w 25q", ""),
        // 9-key PMS (BME-compatible)
        ("9-bme", "11q 12w 13e 14r 15t 18r 19e 16w 17q", ""),
    ];

    /**
     * Determines the key specification from the preset name, in the absence of explicit key
     * specification with `-K` option. (C: `detect_preset`)
     *
     * Besides from presets specified in `PRESETS`, this function also allows the following
     * pseudo-presets inferred from the BMS file:
     *
     * - `bms`, `bme`, `bml` or no preset: Selects one of eight presets `{5,7,10,14}[/fp]`.
     * - `pms`: Selects one of two presets `9` and `9-bme`.
     */
    pub fn preset_to_key_spec(bms: &Bms, preset: Option<String>) -> Option<(String, String)> {
        use std::ascii::OwnedAsciiExt;
        use util::option::StrOption;

        let mut present = [false, ..NLANES];
        for &obj in bms.objs.iter() {
            for &Lane(lane) in obj.object_lane().iter() {
                present[lane] = true;
            }
        }

        let preset = preset.map(|s| s.into_ascii_lower());
        let preset = match preset.as_ref_slice() {
            None | Some("bms") | Some("bme") | Some("bml") => {
                let isbme = present[8] || present[9] || present[36+8] || present[36+9];
                let haspedal = present[7] || present[36+7];
                let nkeys = match bms.player {
                    COUPLE_PLAY | DOUBLE_PLAY => if isbme {"14"} else {"10"},
                    _                         => if isbme {"7" } else {"5" }
                };
                if haspedal {nkeys.to_string() + "/fp"} else {nkeys.to_string()}
            },
            Some("pms") => {
                let isbme = present[6] || present[7] || present[8] || present[9];
                let nkeys = if isbme {"9-bme"} else {"9"};
                nkeys.to_string()
            },
            Some(_) => preset.unwrap()
        };

        for &(name, leftkeys, rightkeys) in PRESETS.iter() {
            if name == preset[] {
                return Some((leftkeys.to_string(), rightkeys.to_string()));
            }
        }
        None
    }

    //----------------------------------------------------------------------------------------------
    // post-processing

    /// Updates the object in place to BGM or placeholder. (C: `remove_or_replace_note`)
    fn remove_or_replace_note(obj: &mut Obj) {
        obj.data = match obj.data {
            Visible(_,Some(sref)) | Invisible(_,Some(sref)) |
            LNStart(_,Some(sref)) | LNDone(_,Some(sref)) => BGM(sref),
            _ => Deleted
        };
    }

    /// Fixes a problematic data. (C: `sanitize_bms`)
    pub fn sanitize_bms(bms: &mut Bms) {
        bms.objs.sort_by(|a, b| {
            if a.time < b.time {Less} else if a.time > b.time {Greater} else {Equal}
        });

        fn sanitize(objs: &mut [Obj], to_type: |&Obj| -> Option<uint>,
                    merge_types: |int| -> int) {
            let len = objs.len();
            let mut i = 0;
            while i < len {
                let cur = objs[i].time;
                let mut types = 0;
                let mut j = i;
                while j < len && objs[j].time <= cur {
                    let obj = &mut objs[j];
                    for &t in to_type(obj).iter() {
                        if (types & (1 << t)) != 0 {
                            // duplicate type
                            remove_or_replace_note(obj);
                        } else {
                            types |= 1 << t;
                        }
                    }
                    j += 1;
                }

                types = merge_types(types);

                while i < j {
                    let obj = &mut objs[i];
                    for &t in to_type(obj).iter() {
                        if (types & (1 << t)) == 0 {
                            remove_or_replace_note(obj);
                        }
                    }
                    i += 1;
                }
            }
        }

        for lane in range(0, NLANES) {
            let lane0 = Lane(lane);

            const LNDONE: uint = 0;
            const LNSTART: uint = 1;
            const VISIBLE: uint = 2;
            const INVISIBLE: uint = 3;
            const BOMB: uint = 4;
            let to_type = |obj: &Obj| -> Option<uint> {
                match obj.data {
                    Visible(lane,_) if lane == lane0 => Some(VISIBLE),
                    Invisible(lane,_) if lane == lane0 => Some(INVISIBLE),
                    LNStart(lane,_) if lane == lane0 => Some(LNSTART),
                    LNDone(lane,_) if lane == lane0 => Some(LNDONE),
                    Bomb(lane,_,_) if lane == lane0 => Some(BOMB),
                    _ => None,
                }
            };

            let mut inside = false;
            sanitize(bms.objs[mut], |obj| to_type(obj), |mut types| {
                const LNMASK: int = (1 << LNSTART) | (1 << LNDONE);

                // remove overlapping LN endpoints altogether
                if (types & LNMASK) == LNMASK { types &= !LNMASK; }

                // remove prohibited types according to inside
                if inside {
                    types &= !((1 << LNSTART) | (1 << VISIBLE) | (1 << BOMB));
                } else {
                    types &= !(1 << LNDONE);
                }

                // invisible note cannot overlap with long note endpoints
                if (types & LNMASK) != 0 { types &= !(1 << INVISIBLE); }

                // keep the most important (lowest) type, except for
                // BOMB/INVISIBLE combination
                let lowest = types & -types;
                if lowest == (1 << INVISIBLE) {
                    types = lowest | (types & (1 << BOMB));
                } else {
                    types = lowest;
                }

                if (types & (1 << LNSTART)) != 0 {
                    inside = true;
                } else if (types & (1 << LNDONE)) != 0 {
                    inside = false;
                }

                types
            });

            if inside {
                // remove last starting longnote which is unfinished
                match bms.objs.iter().rposition(|obj| to_type(obj).is_some()) {
                    Some(pos) if bms.objs[pos].is_lnstart() =>
                        remove_or_replace_note(&mut bms.objs[mut][pos]),
                    _ => {}
                }
            }
        }

        sanitize(bms.objs[mut],
                 |&obj| match obj.data {
                            SetBGA(Layer1,_) => Some(0),
                            SetBGA(Layer2,_) => Some(1),
                            SetBGA(Layer3,_) => Some(2),
                            SetBGA(PoorBGA,_) => Some(3),
                            SetBPM(..) => Some(4),
                            Stop(..) => Some(5),
                            _ => None,
                        },
                 |types| types);
    }

    /// Removes insignificant objects (i.e. not in visible lanes) and ensures that there is no
    /// `Deleted` object. (C: `analyze_and_compact_bms`)
    pub fn compact_bms(bms: &mut Bms, keyspec: &KeySpec) {
        for obj in bms.objs.iter_mut() {
            for &Lane(lane) in obj.object_lane().iter() {
                if keyspec.kinds[lane].is_none() {
                    remove_or_replace_note(obj)
                }
            }
        }

        bms.objs.retain(|obj| obj.data != Deleted);
    }

    //----------------------------------------------------------------------------------------------
    // analysis

    /// Derived BMS information. Again, this is not a global state.
    pub struct BmsInfo {
        /// The start position of the BMS file. This is either -1.0 or 0.0 depending on the first
        /// measure has any visible objects or not. (C: `originoffset`)
        pub originoffset: f64,
        /// Set to true if the BMS file has a BPM change. (C: `hasbpmchange`)
        pub hasbpmchange: bool,
        /// Set to true if the BMS file has long note objects. (C: `haslongnote`)
        pub haslongnote: bool,
        /// The number of visible objects in the BMS file. A long note object counts as one object.
        /// (C: `nnotes`)
        pub nnotes: int,
        /// The maximum possible score. (C: `maxscore`)
        pub maxscore: int
    }

    /// Analyzes the loaded BMS file. (C: `analyze_and_compact_bms`)
    pub fn analyze_bms(bms: &Bms) -> BmsInfo {
        let mut infos = BmsInfo { originoffset: 0.0, hasbpmchange: false, haslongnote: false,
                                  nnotes: 0, maxscore: 0 };

        for &obj in bms.objs.iter() {
            infos.haslongnote |= obj.is_lnstart();
            infos.hasbpmchange |= obj.is_setbpm();

            if obj.is_lnstart() || obj.is_visible() {
                infos.nnotes += 1;
                if obj.time < 1.0 { infos.originoffset = -1.0; }
            }
        }

        for i in range(0, infos.nnotes) {
            let ratio = (i as f64) / (infos.nnotes as f64);
            infos.maxscore += (300.0 * (1.0 + ratio)) as int;
        }

        infos
    }

    /// Calculates the duration of the loaded BMS file in seconds. `sound_length` should return
    /// the length of sound resources in seconds or 0.0. (C: `get_bms_duration`)
    pub fn bms_duration(bms: &Bms, originoffset: f64,
                        sound_length: |SoundRef| -> f64) -> f64 {
        let mut pos = originoffset;
        let mut bpm = bms.initbpm;
        let mut time = 0.0;
        let mut sndtime = 0.0;

        for &obj in bms.objs.iter() {
            let delta = bms.adjust_object_position(pos, obj.time);
            time += bpm.measure_to_msec(delta);
            match obj.data {
                Visible(_,Some(sref)) | LNStart(_,Some(sref)) | BGM(sref) => {
                    let sndend = time + sound_length(sref) * 1000.0;
                    if sndtime > sndend { sndtime = sndend; }
                }
                SetBPM(BPM(newbpm)) => {
                    if newbpm > 0.0 {
                        bpm = BPM(newbpm);
                    } else if newbpm < 0.0 {
                        bpm = BPM(newbpm);
                        let delta = bms.adjust_object_position(originoffset, pos);
                        time += BPM(-newbpm).measure_to_msec(delta);
                        break;
                    }
                }
                Stop(duration) => {
                    time += duration.to_msec(bpm);
                }
                _ => {}
            }
            pos = obj.time;
        }

        if *bpm > 0.0 { // the chart scrolls backwards to `originoffset` for negative BPM
            let delta = bms.adjust_object_position(pos, (bms.nmeasures + 1) as f64);
            time += bpm.measure_to_msec(delta);
        }
        (if time > sndtime {time} else {sndtime}) / 1000.0
     }

    //----------------------------------------------------------------------------------------------
    // modifiers

    /// Applies a function to the object lane if any. This is used to shuffle the lanes without
    /// modifying the relative time position.
    fn update_object_lane(obj: &mut Obj, f: |Lane| -> Lane) {
        obj.data = match obj.data {
            Visible(lane,sref) => Visible(f(lane),sref),
            Invisible(lane,sref) => Invisible(f(lane),sref),
            LNStart(lane,sref) => LNStart(f(lane),sref),
            LNDone(lane,sref) => LNDone(f(lane),sref),
            Bomb(lane,sref,damage) => Bomb(f(lane),sref,damage),
            objdata => objdata
        };
    }

    /// Swaps given lanes in the reverse order. (C: `shuffle_bms` with `MIRROR_MODF`)
    pub fn apply_mirror_modf(bms: &mut Bms, lanes: &[Lane]) {
        let mut map = Vec::from_fn(NLANES, |lane| Lane(lane));
        for (&Lane(from), &to) in lanes.iter().zip(lanes.iter().rev()) {
            map[mut][from] = to;
        }

        for obj in bms.objs.iter_mut() {
            update_object_lane(obj, |Lane(lane)| map[lane]);
        }
    }

    /// Swaps given lanes in the random order. (C: `shuffle_bms` with
    /// `SHUFFLE_MODF`/`SHUFFLEEX_MODF`)
    pub fn apply_shuffle_modf<R:Rng>(bms: &mut Bms, r: &mut R, lanes: &[Lane]) {
        let mut shuffled = lanes.to_vec();
        r.shuffle(shuffled[mut]);
        let mut map = Vec::from_fn(NLANES, |lane| Lane(lane));
        for (&Lane(from), &to) in lanes.iter().zip(shuffled.iter()) {
            map[mut][from] = to;
        }

        for obj in bms.objs.iter_mut() {
            update_object_lane(obj, |Lane(lane)| map[lane]);
        }
    }

    /// Swaps given lanes in the random order, where the order is determined per object.
    /// `bms` should be first sanitized by `sanitize_bms`. It does not cause objects to move within
    /// another LN object, or place two objects in the same or very close time position to the same
    /// lane. (C: `shuffle_bms` with `RANDOM_MODF`/`RANDOMEX_MODF`)
    pub fn apply_random_modf<R:Rng>(bms: &mut Bms, r: &mut R, lanes: &[Lane]) {
        let mut movable = lanes.to_vec();
        let mut map = Vec::from_fn(NLANES, |lane| Lane(lane));

        let mut lasttime = f64::NEG_INFINITY;
        for obj in bms.objs.iter_mut() {
            if obj.is_lnstart() {
                let lane = obj.object_lane().unwrap();
                match movable.iter().position(|&i| i == lane) {
                    Some(i) => { movable.swap_remove(i); }
                    None => panic!("non-sanitized BMS detected")
                }
            }
            if lasttime < obj.time { // reshuffle required
                lasttime = obj.time + 1e-4;
                let mut shuffled = movable.clone();
                r.shuffle(shuffled[mut]);
                for (&Lane(from), &to) in movable.iter().zip(shuffled.iter()) {
                    map[mut][from] = to;
                }
            }
            if obj.is_lnstart() {
                let lane = obj.object_lane().unwrap();
                movable.push(lane);
            }
            update_object_lane(obj, |Lane(lane)| map[lane]);
        }
    }

    //----------------------------------------------------------------------------------------------

}

//==================================================================================================
// graphics

/// Graphic utilities.
pub mod gfx {
    use std;
    use std::{num, cmp};
    use sdl::Rect;
    use sdl::video;
    use sdl::video::{Color, RGB, RGBA, Surface};

    //----------------------------------------------------------------------------------------------
    // `Rect` additions

    /// A trait that can be translated to point coordinates (`x` and `y` fields in `sdl::Rect`,
    /// hence the name). Also contains `()`.
    pub trait XyOpt {
        /// Returns point coordinates if any.
        fn xy_opt(&self) -> Option<(i16,i16)>;
    }

    /// Same as `XyOpt` but does not contain `()`.
    pub trait Xy: XyOpt {
        /// Returns point coordinates.
        fn xy(&self) -> (i16,i16);
    }

    /// A trait that can be translated to a rectangular area (`w` and `h` fields in `sdl::Rect`,
    /// hence the name). Also contains `()`.
    pub trait WhOpt {
        /// Returns a rectangular area if any.
        fn wh_opt(&self) -> Option<(u16,u16)>;
    }

    /// Same as `WhOpt` but does not contain `()`.
    pub trait Wh {
        /// Returns a rectangular area.
        fn wh(&self) -> (u16,u16);
    }

    impl XyOpt for () {
        #[inline(always)]
        fn xy_opt(&self) -> Option<(i16,i16)> { None }
    }

    // Rust: we can't define these with `impl<T:Xy> XyOpt for T` due to the ambiguity.
    impl XyOpt for Rect {
        #[inline(always)]
        fn xy_opt(&self) -> Option<(i16,i16)> { Some((self.x, self.y)) }
    }

    impl<'r,T:XyOpt> XyOpt for &'r T {
        #[inline(always)]
        fn xy_opt(&self) -> Option<(i16,i16)> { (*self).xy_opt() }
    }

    impl Xy for Rect {
        #[inline(always)]
        fn xy(&self) -> (i16,i16) { (self.x, self.y) }
    }

    impl<'r,T:Xy> Xy for &'r T {
        #[inline(always)]
        fn xy(&self) -> (i16,i16) { (*self).xy() }
    }

    impl WhOpt for () {
        #[inline(always)]
        fn wh_opt(&self) -> Option<(u16,u16)> { None }
    }

    impl WhOpt for Rect {
        #[inline(always)]
        fn wh_opt(&self) -> Option<(u16,u16)> { Some((self.w, self.h)) }
    }

    impl WhOpt for Surface {
        #[inline(always)]
        fn wh_opt(&self) -> Option<(u16,u16)> { Some(self.get_size()) }
    }

    impl<'r,T:WhOpt> WhOpt for &'r T {
        #[inline(always)]
        fn wh_opt(&self) -> Option<(u16,u16)> { (*self).wh_opt() }
    }

    impl Wh for Rect {
        #[inline(always)]
        fn wh(&self) -> (u16,u16) { (self.w, self.h) }
    }

    impl Wh for Surface {
        #[inline(always)]
        fn wh(&self) -> (u16,u16) { self.get_size() }
    }

    impl<'r,T:Wh> Wh for &'r T {
        #[inline(always)]
        fn wh(&self) -> (u16,u16) { (*self).wh() }
    }

    /// A helper trait for defining every implementations for types `(T1,T2)` where `T1` and `T2` is
    /// convertible to an integer.
    trait ToInt16 {
        /// Converts to `i16`.
        fn to_i16(&self) -> i16;
        /// Converts to `u16`.
        fn to_u16(&self) -> u16;
    }

    macro_rules! define_ToInt16(
        ($t:ty) => (impl ToInt16 for $t {
                        #[inline(always)]
                        fn to_i16(&self) -> i16 { *self as i16 }
                        #[inline(always)]
                        fn to_u16(&self) -> u16 { *self as u16 }
                    })
    )

    define_ToInt16!(int)
    define_ToInt16!(uint)
    define_ToInt16!(i8)
    define_ToInt16!(i16)
    define_ToInt16!(i32)
    define_ToInt16!(i64)
    define_ToInt16!(u8)
    define_ToInt16!(u16)
    define_ToInt16!(u32)
    define_ToInt16!(u64)

    impl<X:ToInt16+Clone,Y:ToInt16+Clone> XyOpt for (X,Y) {
        #[inline(always)]
        fn xy_opt(&self) -> Option<(i16,i16)> {
            let (x, y) = self.clone();
            Some((x.to_i16(), y.to_i16()))
        }
    }

    impl<X:ToInt16+Clone,Y:ToInt16+Clone> Xy for (X,Y) {
        #[inline(always)]
        fn xy(&self) -> (i16,i16) {
            let (x, y) = self.clone();
            (x.to_i16(), y.to_i16())
        }
    }

    impl<W:ToInt16+Clone,H:ToInt16+Clone> WhOpt for (W,H) {
        #[inline(always)]
        fn wh_opt(&self) -> Option<(u16,u16)> {
            let (w, h) = self.clone();
            Some((w.to_u16(), h.to_u16()))
        }
    }

    impl<W:ToInt16+Clone,H:ToInt16+Clone> Wh for (W,H) {
        #[inline(always)]
        fn wh(&self) -> (u16,u16) {
            let (w, h) = self.clone();
            (w.to_u16(), h.to_u16())
        }
    }

    /// Constructs an `sdl::Rect` from given point coordinates. Fills `w` and `h` fields to 0
    /// as expected by the second `sdl::Rect` argument from `SDL_BlitSurface`.
    #[inline(always)]
    pub fn rect_from_xy<XY:Xy>(xy: XY) -> Rect {
        let (x, y) = xy.xy();
        Rect { x: x, y: y, w: 0, h: 0 }
    }

    /// Constructs an `sdl::Rect` from given point coordinates and optional rectangular area.
    /// `rect_from_xywh(xy, ())` equals to `rect_from_xy(xy)`.
    #[inline(always)]
    pub fn rect_from_xywh<XY:Xy,WH:WhOpt>(xy: XY, wh: WH) -> Rect {
        let (x, y) = xy.xy();
        let (w, h) = wh.wh_opt().unwrap_or((0, 0));
        Rect { x: x, y: y, w: w, h: h }
    }

    /// Additions to `sdl::video::Surface`. They replace their `_rect` suffixed counterparts,
    /// which are generally annoying to work with.
    pub trait SurfaceAreaUtil {
        /// An alternative interface to `set_clip_rect`.
        fn set_clip_area<XY:Xy,WH:WhOpt>(&self, xy: XY, wh: WH);
        /// An alternative interface to `blit_rect`.
        fn blit_area<SrcXY:Xy,DstXY:XyOpt,WH:WhOpt>(&self, src: &Surface,
                                                    srcxy: SrcXY, dstxy: DstXY, wh: WH) -> bool;
        /// An alternative interface to `fill_rect`.
        fn fill_area<XY:Xy,WH:WhOpt>(&self, xy: XY, wh: WH, color: Color) -> bool;
    }

    impl SurfaceAreaUtil for Surface {
        #[inline(always)]
        fn set_clip_area<XY:Xy,WH:WhOpt>(&self, xy: XY, wh: WH) {
            let rect = rect_from_xywh(xy, wh);
            self.set_clip_rect(&rect)
        }

        #[inline(always)]
        fn blit_area<SrcXY:Xy,DstXY:XyOpt,WH:WhOpt>(&self, src: &Surface,
                                                    srcxy: SrcXY, dstxy: DstXY, wh: WH) -> bool {
            let srcrect = rect_from_xywh(srcxy, wh);
            let dstrect = dstxy.xy_opt().map(|xy| rect_from_xywh(xy, &srcrect));
            self.blit_rect(src, Some(srcrect), dstrect)
        }

        #[inline(always)]
        fn fill_area<XY:Xy,WH:WhOpt>(&self, xy: XY, wh: WH, color: Color) -> bool {
            let rect = rect_from_xywh(xy, wh);
            self.fill_rect(Some(rect), color)
        }
    }

    //----------------------------------------------------------------------------------------------
    // color

    /// Extracts red, green, blue components from given color.
    fn to_rgb(c: Color) -> (u8, u8, u8) {
        match c { RGB(r, g, b) | RGBA(r, g, b, _) => (r, g, b) }
    }

    /// Linear color gradient.
    #[deriving(PartialEq)]
    pub struct Gradient {
        /// A color at the position 0.0. Normally used as a topmost value.
        pub zero: Color,
        /// A color at the position 1.0. Normally used as a bottommost value.
        pub one: Color
    }

    impl Gradient {
        /// Creates a new color gradient (for text printing).
        pub fn new(top: Color, bottom: Color) -> Gradient {
            Gradient { zero: top, one: bottom }
        }
    }

    /// A trait for color or color gradient. The color at the particular position can be calculated
    /// with `blend` method.
    pub trait Blend {
        /// Returns itself. This is same as `Clone::clone` but redefined here due to the inability
        /// of implementing `Clone` for `Color`.
        fn clone(&self) -> Self;
        /// Calculates the color at the position `num/denom`. (C: `blend`)
        fn blend(&self, num: int, denom: int) -> Color;
    }

    impl Blend for Color {
        fn clone(&self) -> Color { *self }
        fn blend(&self, _num: int, _denom: int) -> Color { *self }
    }

    impl Blend for Gradient {
        fn clone(&self) -> Gradient { *self }
        fn blend(&self, num: int, denom: int) -> Color {
            fn mix(x: u8, y: u8, num: int, denom: int) -> u8 {
                let x = x as int;
                let y = y as int;
                (y + ((x - y) * num / denom)) as u8
            }

            let (r0, g0, b0) = to_rgb(self.zero);
            let (r1, g1, b1) = to_rgb(self.one);
            RGB(mix(r1, r0, num, denom), mix(g1, g0, num, denom), mix(b1, b0, num, denom))
        }
    }

    //----------------------------------------------------------------------------------------------
    // surface utilities

    /// Creates a new RAM-backed surface. By design, Angolmois does not use a VRAM-backed surface
    /// except for the screen. (C: `newsurface`)
    pub fn new_surface(w: uint, h: uint) -> Surface {
        match Surface::new([video::SWSurface], w as int, h as int, 32, 0xff0000, 0xff00, 0xff, 0) {
            Ok(surface) => surface,
            Err(err) => die!("new_surface failed: {}", err)
        }
    }

    /// A proxy to `sdl::video::Surface` for the direct access to pixels. For now, it is for 32 bits
    /// per pixel only.
    pub struct SurfacePixels<'r> {
        fmt: *mut video::ll::SDL_PixelFormat,
        width: uint,
        height: uint,
        pitch: uint,
        pixels: &'r mut [u32]
    }

    /// A trait for the direct access to pixels.
    pub trait SurfacePixelsUtil {
        /// Grants the direct access to pixels. Also locks the surface as needed, so you can't blit
        /// during working with pixels.
        fn with_pixels<R>(&self, f: |pixels: &mut SurfacePixels| -> R) -> R;
    }

    impl SurfacePixelsUtil for Surface {
        fn with_pixels<R>(&self, f: |pixels: &mut SurfacePixels| -> R) -> R {
            self.with_lock(|pixels| {
                let fmt = unsafe {(*self.raw).format};
                let pitch = unsafe {((*self.raw).pitch / 4) as uint};
                let pixels = unsafe {std::mem::transmute(pixels)};
                let mut proxy = SurfacePixels { fmt: fmt, width: self.get_width() as uint,
                                                height: self.get_height() as uint,
                                                pitch: pitch, pixels: pixels };
                f(&mut proxy)
            })
        }
    }

    impl<'r> SurfacePixels<'r> {
        /// Returns a pixel at given position. (C: `getpixel`)
        pub fn get_pixel(&self, x: uint, y: uint) -> Color {
            Color::from_mapped(self.pixels[x + y * self.pitch], self.fmt as *const _)
        }

        /// Sets a pixel to given position. (C: `putpixel`)
        pub fn put_pixel(&mut self, x: uint, y: uint, c: Color) {
            self.pixels[x + y * self.pitch] = c.to_mapped(self.fmt as *const _);
        }

        /// Sets or blends (if `c` is `RGBA`) a pixel to given position. (C: `putblendedpixel`)
        pub fn put_blended_pixel(&mut self, x: uint, y: uint, c: Color) {
            match c {
                RGB(..) => self.put_pixel(x, y, c),
                RGBA(r,g,b,a) => match self.get_pixel(x, y) {
                    RGB(r2,g2,b2) | RGBA(r2,g2,b2,_) => {
                        let grad = Gradient { zero: RGB(r,g,b), one: RGB(r2,g2,b2) };
                        self.put_pixel(x, y, grad.blend(a as int, 255));
                    }
                }
            }
        }
    }

    /// A scaling factor for the calculation of convolution kernel.
    const FP_SHIFT1: uint = 11;
    /// A scaling factor for the summation of weighted pixels.
    const FP_SHIFT2: uint = 16;

    /// Returns `2^FP_SHIFT * W(x/y)` where `W(x)` is a bicubic kernel function. `y` should be
    /// positive. (C: `bicubic_kernel`)
    fn bicubic_kernel(x: int, y: int) -> int {
        let x = num::abs(x);
        if x < y {
            // W(x/y) = 1/2 (2 - 5(x/y)^2 + 3(x/y)^3)
            ((2*y*y - 5*x*x + 3*x*x/y*x) << (FP_SHIFT1-1)) / (y*y)
        } else if x < y * 2 {
            // W(x/y) = 1/2 (4 - 8(x/y) + 5(x/y)^2 - (x/y)^3)
            ((4*y*y - 8*x*y + 5*x*x - x*x/y*x) << (FP_SHIFT1-1)) / (y*y)
        } else {
            0
        }
    }

    /**
     * Performs the bicubic interpolation. `dest` should be initialized to the target dimension
     * before calling this function. This function should be used only for the upscaling; it can do
     * the downscaling somehow but technically its result is incorrect. (C: `bicubic_interpolation`)
     *
     * Well, this function is one of the ugliest functions in Angolmois, especially since it is
     * a complicated (in terms of code complexity) and still poor (we normally use the matrix form
     * instead) implementation of the algorithm. In fact, the original version of `bicubic_kernel`
     * had even a slightly incorrect curve (`1/2 - x^2 + 1/2 x^3` instead of `1 - 5/2 x^2 +
     * 3/2 x^3`). This function still remains here only because we don't use OpenGL...
     */
    pub fn bicubic_interpolation(src: &SurfacePixels, dest: &mut SurfacePixels) {
        let w = dest.width as int - 1;
        let h = dest.height as int - 1;
        let ww = src.width as int - 1;
        let hh = src.height as int - 1;

        let mut dx = 0;
        let mut x = 0;
        for i in range(0, w + 1) {
            let mut dy = 0;
            let mut y = 0;
            for j in range(0, h + 1) {
                let mut r = 0;
                let mut g = 0;
                let mut b = 0;
                let a0 = [bicubic_kernel((x-1) * w - i * ww, w),
                          bicubic_kernel( x    * w - i * ww, w),
                          bicubic_kernel((x+1) * w - i * ww, w),
                          bicubic_kernel((x+2) * w - i * ww, w)];
                let a1 = [bicubic_kernel((y-1) * h - j * hh, h),
                          bicubic_kernel( y    * h - j * hh, h),
                          bicubic_kernel((y+1) * h - j * hh, h),
                          bicubic_kernel((y+2) * h - j * hh, h)];
                for k0 in range(0u, 4) {
                    for k1 in range(0u, 4) {
                        let xx = x + k0 as int - 1;
                        let yy = y + k1 as int - 1;
                        if 0 <= xx && xx <= ww && 0 <= yy && yy <= hh {
                            let (r2,g2,b2) = to_rgb(src.get_pixel(xx as uint, yy as uint));
                            let d = (a0[k0] * a1[k1]) >> (FP_SHIFT1*2 - FP_SHIFT2);
                            r += r2 as int * d;
                            g += g2 as int * d;
                            b += b2 as int * d;
                        }
                    }
                }

                let r = cmp::min(cmp::max(r >> FP_SHIFT2, 0), 255) as u8;
                let g = cmp::min(cmp::max(g >> FP_SHIFT2, 0), 255) as u8;
                let b = cmp::min(cmp::max(b >> FP_SHIFT2, 0), 255) as u8;
                dest.put_pixel(i as uint, j as uint, RGB(r, g, b));

                dy += hh;
                if dy > h {
                    y += 1;
                    dy -= h;
                }
            }

            dx += ww;
            if dx > w {
                x += 1;
                dx -= w;
            }
        }
    }

    //----------------------------------------------------------------------------------------------
    // bitmap font

    /// Bit vector which represents one row of zoomed font.
    type ZoomedFontRow = u32;

    /// 8x16 resizable bitmap font.
    pub struct Font {
        /**
         * Font data used for zoomed font reconstruction. This is actually an array of `u32`
         * elements, where the first `u16` element forms upper 16 bits and the second forms lower
         * 16 bits. It is reinterpreted for better compression. (C: `fontdata`)
         *
         * One glyph has 16 `u32` elements for each row from the top to the bottom. One `u32`
         * element contains eight four-bit groups for each column from the left (lowermost group)
         * to the right (uppermost group). Each group is a bitwise OR of following bits:
         *
         * - 1: the lower right triangle of the zoomed pixel should be drawn.
         * - 2: the lower left triangle of the zoomed pixel should be drawn.
         * - 4: the upper left triangle of the zoomed pixel should be drawn.
         * - 8: the upper right triangle of the zoomed pixel should be drawn.
         *
         * So for example, if the group bits read 3 (1+2), the zoomed pixel would be drawn
         * as follows (in the zoom factor 5):
         *
         *     .....
         *     #...#
         *     ##.##
         *     #####
         *     #####
         *
         * The group bits 15 (1+2+4+8) always draw the whole square, so in the zoom factor 1 only
         * pixels with group bits 15 will be drawn.
         */
        glyphs: Vec<u16>,

        /// Precalculated zoomed font per zoom factor. It is three-dimensional array which indices
        /// are zoom factor, glyph number and row respectively. Assumes that each element has
        /// at least zoom factor times 8 (columns per row) bits. (C: `zoomfont`)
        pixels: Vec<Vec<Vec<ZoomedFontRow>>>
    }

    /// An alignment mode of `Font::print_string`.
    pub enum Alignment {
        /// Coordinates specify the top-left corner of the bounding box.
        LeftAligned,
        /// Coordinates specify the top-center point of the bounding box.
        Centered,
        /// Coordinates specify the top-right corner of the bounding box.
        RightAligned
    }

    // Delta-coded code words. (C: `words`)
    static FONT_DWORDS: &'static [u16] = &[
        0, 2, 6, 2, 5, 32, 96, 97, 15, 497, 15, 1521, 15, 1537,
        16, 48, 176, 1, 3, 1, 3, 7, 1, 4080, 4096, 3, 1, 8, 3, 4097, 4080,
        16, 16128, 240, 1, 2, 9, 3, 8177, 15, 16385, 240, 15, 1, 47, 721,
        143, 2673, 2, 6, 7, 1, 31, 17, 16, 63, 64, 33, 0, 1, 2, 1, 8, 3];

    // LZ77-compressed indices to code words:
    // - Byte 33..97 encodes a literal code word 0..64;
    // - Byte 98..126 encodes an LZ77 length distance pair with length 3..31;
    //   the following byte 33..126 encodes a distance 1..94.
    // (C: `indices`)
    static FONT_INDICES: &'static [u8] =
        b"!!7a/&/&s$7a!f!'M*Q*Qc$(O&J!!&J&Jc(e!2Q2Qc$-Bg2m!2bB[Q7Q2[e&2Q!Qi>&!&!>UT2T2&2>WT!c*\
          T2GWc8icM2U2D!.8(M$UQCQ-jab!'U*2*2*2TXbZ252>9ZWk@*!*!*8(J$JlWi@cxQ!Q!d$#Q'O*?k@e2dfe\
          jcNl!&JTLTLG_&J>]c*&Jm@cB&J&J7[e(o>pJM$Qs<7[{Zj`Jm40!3!.8(M$U!C!-oR>UQ2U2]2a9Y[S[QCQ\
          2GWk@*M*Q*B*!*!g$aQs`G8.M(U$[!Ca[o@Q2Q!IJQ!Q!c,GWk@787M6U2C2d!a[2!2k?!bnc32>[u`>Uc4d\
          @b(q@abXU!D!.8(J&J&d$q`Q2IXu`g@Q2aWQ!q@!!ktk,x@M$Qk@3!.8(M$U!H#W'O,?4m_f!7[i&n!:eX5g\
          hCk=>UQ2Q2U2Dc>J!!&J&b&k@J)LKg!GK!)7Wk@'8,M=UWCcfa[c&Q2l`f4If(Q2G[l@MSUQC!2!2c$Q:RWG\
          Ok@,[<2WfZQ2U2D2.l`a[eZ7f(!2b2|@b$j!>MSUQCc6[2W2Q:RWGOk@Q2Q2c$a[g*Ql`7[&J&Jk$7[l`!Qi\
          $d^GWk@U2D2.9([$[#['[,@<2W2k@!2!2m$a[l`:^[a[a[T2Td~c$k@d2:R[V[a@_b|o@,M=UWCgZU:EW.Ok\
          @>[g<G[!2!2d$k@Ug@Q2V2a2IW_!Wt`Ih*q`!2>WQ!Q!c,Gk_!7[&J&Jm$k@gti$m`k:U:EW.O(?s@T2Tb$a\
          [CW2Qk@M+U:^[GbX,M>U`[WCO-l@'U,D<.W(O&J&Je$k@a[Q!U!]!G8.M(U$[!Ca[k@*Q!Q!l$b2m!+!:#W'\
          O,?4!1n;c`*!*!l$h`'8,M=UWCO-pWz!a[i,#Q'O,?4~R>QQ!Q!aUQ2Q2Q2aWl=2!2!2>[e<c$G[p`dZcHd@\
          l`czi|c$al@i`b:[!2Un`>8TJTJ&J7[&b&e$o`i~aWQ!c(hd2!2!2>[g@e$k]epi|e0i!bph(d$dbGWhA2!2\
          U2D2.9(['[,@<2W2k`*J*?*!*!k$o!;[a[T2T2c$c~o@>[c6i$p@Uk>GW}`G[!2!2b$h!al`aWQ!Q!Qp`fVl\
          Zf@UWb6>eX:GWk<&J&J7[c&&JTJTb$G?o`c~i$m`k@U:EW.O(v`T2Tb$a[Fp`M+eZ,M=UWCO-u`Q:RWGO.A(\
          M$U!Ck@a[]!G8.M(U$[!Ca[i:78&J&Jc$%[g*7?e<g0w$cD#iVAg*$[g~dB]NaaPGft~!f!7[.W(O";

    impl Font {
        /// Decompresses a bitmap font data.
        /// `Font::create_zoomed_font` is required for the actual use.
        pub fn new() -> Font {
            /// Decompresses a font data from `dwords` and `indices`. (C: `fontdecompress`)
            fn decompress(dwords: &[u16], indices: &[u8]) -> Vec<u16> {
                let mut words = vec!(0);
                for &delta in dwords.iter() {
                    let last = *words.last().unwrap();
                    words.push(last + delta);
                }

                let nindices = indices.len();
                let mut i = 0;
                let mut glyphs = Vec::new();
                while i < nindices {
                    let code = indices[i] as uint;
                    i += 1;
                    match code {
                        33...97 => { glyphs.push(words[code - 33]); }
                        98...126 => {
                            let length = code - 95; // code=98 -> length=3
                            let distance = indices[i] as uint - 32;
                            i += 1;
                            let start = glyphs.len() - distance;
                            for i in range(start, start + length) {
                                let v = glyphs[i];
                                glyphs.push(v);
                            }
                        }
                        _ => panic!("unexpected codeword")
                    }
                }
                glyphs
            }

            let glyphs = decompress(FONT_DWORDS, FONT_INDICES);
            assert!(glyphs.len() == 3072);
            Font { glyphs: glyphs, pixels: Vec::new() }
        }

        /// Creates a zoomed font of scale `zoom`. (C: `fontprocess`)
        pub fn create_zoomed_font(&mut self, zoom: uint) {
            assert!(zoom > 0);
            assert!(zoom <= (8 * std::mem::size_of::<ZoomedFontRow>()) / 8);
            if zoom < self.pixels.len() && !self.pixels[zoom].is_empty() { return; }

            let nrows = 16;
            let nglyphs = self.glyphs.len() / nrows / 2;
            let mut pixels = Vec::from_elem(nglyphs, Vec::from_elem(zoom*nrows, 0u32));

            let put_zoomed_pixel = |pixels: &mut Vec<Vec<ZoomedFontRow>>,
                                    glyph: uint, row: uint, col: uint, v: u32| {
                let zoomrow = row * zoom;
                let zoomcol = col * zoom;
                for r in range(0, zoom) {
                    for c in range(0, zoom) {
                        let mut mask = 0;
                        if r + c >= zoom    { mask |= 1; } // lower right
                        if r > c            { mask |= 2; } // lower left
                        if r < c            { mask |= 4; } // upper right
                        if r + c < zoom - 1 { mask |= 8; } // upper left

                        // if `zoom` is odd, drawing four corner triangles leaves one center pixel
                        // intact since we don't draw diagonals for aesthetic reason. such case
                        // must be specially handled.
                        if (v & mask) != 0 || v == 15 {
                            pixels[mut][glyph][mut][zoomrow+r] |= 1 << (zoomcol+c);
                        }
                    }
                }
            };

            let mut i = 0;
            for glyph in range(0, nglyphs) {
                for row in range(0, nrows) {
                    let data = (self.glyphs[i] as u32 << 16) | (self.glyphs[i+1] as u32);
                    i += 2;
                    for col in range(0, 8u) {
                        let v = (data >> (4 * col)) & 15;
                        put_zoomed_pixel(&mut pixels, glyph, row, col, v);
                    }
                }
            }
            if self.pixels.len() <= zoom {
                let ncopies = zoom - self.pixels.len() + 1;
                self.pixels.grow(ncopies, Vec::new());
            }
            self.pixels[mut][zoom] = pixels;
        }

        /// Prints a glyph with given position and color (possibly gradient). This method is
        /// distinct from `print_glyph` since the glyph #95 is used for the tick marker
        /// (character code -1 in C). (C: `printchar`)
        pub fn print_glyph<ColorT:Blend>(&self, pixels: &mut SurfacePixels, x: uint, y: uint,
                                         zoom: uint, glyph: uint, color: ColorT) {
            assert!(!self.pixels[zoom].is_empty());
            for iy in range(0, 16 * zoom) {
                let row = self.pixels[zoom][glyph][iy];
                let rowcolor = color.blend(iy as int, 16 * zoom as int);
                for ix in range(0, 8 * zoom) {
                    if ((row >> ix) & 1) != 0 {
                        pixels.put_pixel(x + ix, y + iy, rowcolor);
                    }
                }
            }
        }

        /// Prints a character with given position and color.
        pub fn print_char<ColorT:Blend>(&self, pixels: &mut SurfacePixels, x: uint, y: uint,
                                        zoom: uint, c: char, color: ColorT) {
            if !c.is_whitespace() {
                let c = c as uint;
                let glyph = if 32 <= c && c < 126 {c-32} else {0};
                self.print_glyph(pixels, x, y, zoom, glyph, color);
            }
        }

        /// Prints a string with given position, alignment and color. (C: `printstr`)
        pub fn print_string<ColorT:Blend>(&self, pixels: &mut SurfacePixels, x: uint, y: uint,
                                          zoom: uint, align: Alignment, s: &str, color: ColorT) {
            let mut x = match align {
                LeftAligned  => x,
                Centered     => x - s.char_len() * (8 * zoom) / 2,
                RightAligned => x - s.char_len() * (8 * zoom),
            };
            for c in s.chars() {
                let nextx = x + 8 * zoom;
                if nextx >= pixels.width { break; }
                self.print_char(pixels, x, y, zoom, c, color.clone());
                x = nextx;
            }
        }
    }

    //----------------------------------------------------------------------------------------------

}

//==================================================================================================
// game play

/**
 * Game play logics. This module contains whooping 2000+ lines of code, reflecting the fact that
 * Angolmois is not well refactored. (In fact, the game logic is usually hard to refactor, right?)
 */
pub mod player {
    use {std, libc};
    use std::{slice, cmp, num, iter, hash};
    use std::rc::Rc;
    use std::rand::Rng;
    use std::collections::HashMap;

    use {sdl, sdl_image, sdl_mixer};
    use sdl::{audio, video, event, joy};
    use sdl::video::{RGB, RGBA, Surface, Color};
    use sdl::event::{NoEvent, KeyEvent, JoyButtonEvent, JoyAxisEvent, QuitEvent};
    use sdl_mixer::Chunk;
    use util::smpeg::MPEG;

    use {parser, gfx};
    use parser::{Key, Lane, NLANES, KeyKind, BPM, Damage, GaugeDamage, InstantDeath};
    use parser::{BGALayer, NLAYERS, Layer1, Layer2, Layer3, PoorBGA};
    use parser::{Obj, ObjData, ObjQueryOps, ImageRef, SoundRef, BGM, SetBGA, SetBPM, Stop,
                 Visible, LNStart, LNDone, Bomb};
    use parser::{Bms, BmsInfo, KeySpec, BlitCmd};
    use gfx::{Gradient, Blend, Font, LeftAligned, Centered, RightAligned};
    use gfx::{SurfaceAreaUtil, SurfacePixelsUtil};

    /// The width of screen, unless the exclusive mode.
    pub const SCREENW: uint = 800;
    /// The height of screen, unless the exclusive mode.
    pub const SCREENH: uint = 600;
    /// The width of BGA, or the width of screen for the exclusive mode.
    pub const BGAW: uint = 256;
    /// The height of BGA, or the height of screen for the exclusive mode.
    pub const BGAH: uint = 256;

    //----------------------------------------------------------------------------------------------
    // options

    /// Game play modes. (C: `enum mode`)
    #[deriving(PartialEq,Eq)]
    pub enum Mode {
        /// Normal game play. The graphical display and input is enabled. (C: `PLAY_MODE`)
        PlayMode,
        /// Automatic game play. The graphical display is enabled but the input is mostly ignored
        /// except for the play speed change. (C: `AUTOPLAY_MODE`)
        AutoPlayMode,
        /// Exclusive (headless) mode. The graphical display is reduced to the BGA or absent at all
        /// (when `NoBga` is also set). (C: `EXCLUSIVE_MODE`)
        ExclusiveMode
    }

    /// Modifiers that affect the game data. (C: `enum modf`)
    #[deriving(PartialEq,Eq)]
    pub enum Modf {
        /// Swaps all "key" (i.e. `KeyKind::counts_as_key` returns true) lanes in the reverse order.
        /// See `player::apply_mirror_modf` for the detailed algorithm. (C: `MIRROR_MODF`)
        MirrorModf,
        /// Swaps all "key" lanes in the random order. See `player::apply_shuffle_modf` for
        /// the detailed algorithm. (C: `SHUFFLE_MODF`)
        ShuffleModf,
        /// Swaps all lanes in the random order. (C: `SHUFFLEEX_MODF`)
        ShuffleExModf,
        /// Swaps all "key" lanes in the random order, where the order is determined per object.
        /// See `player::apply_random_modf` for the detailed algorithm. (C: `RANDOM_MODF`)
        RandomModf,
        /// Swaps all lanes in the random order, where the order is determined per object.
        /// (C: `RANDOMEX_MODF`)
        RandomExModf
    }

    /// Specifies how the BGA is displayed. (C: `enum bga`)
    #[deriving(PartialEq,Eq)]
    pub enum Bga {
        /// Both the BGA image and movie is displayed. (C: `BGA_AND_MOVIE`)
        BgaAndMovie,
        /// The BGA is displayed but the movie is not loaded. (C: `BGA_BUT_NO_MOVIE`)
        BgaButNoMovie,
        /// The BGA is not displayed. When used with `ExclusiveMode` it also disables the graphical
        /// display entirely. (C: `NO_BGA`)
        NoBga
    }

    /// Global options set from the command line and environment variables.
    pub struct Options {
        /// A path to the BMS file. Used for finding the resource when `BMS::basepath` is not set.
        /// (C: `bmspath`)
        pub bmspath: String,
        /// Game play mode. (C: `opt_mode`)
        pub mode: Mode,
        /// Modifiers that affect the game data. (C: `opt_modf`)
        pub modf: Option<Modf>,
        /// Specifies how the BGA is displayed. (C: `opt_bga`)
        pub bga: Bga,
        /// True if the metadata (either overlaid in the loading screen or printed separately
        /// in the console) is displayed. (C: `opt_showinfo`)
        pub showinfo: bool,
        /// True if the full screen is enabled. (C: `opt_fullscreen`)
        pub fullscreen: bool,
        /// An index to the joystick device if any. (C: `opt_joystick`)
        pub joystick: Option<uint>,
        /// A key specification preset name if any. (C: `preset`)
        pub preset: Option<String>,
        /// A left-hand-side key specification if any. (C: `leftkeys`)
        pub leftkeys: Option<String>,
        /// A right-hand-side key specification if any. Can be an empty string. (C: `rightkeys`)
        pub rightkeys: Option<String>,
        /// An initial play speed. (C: `playspeed`)
        pub playspeed: f64,
    }

    impl Options {
        /// Returns true if the exclusive mode is enabled. This enables a text-based interface.
        /// (C: `opt_mode >= EXCLUSIVE_MODE`)
        pub fn is_exclusive(&self) -> bool { self.mode == ExclusiveMode }

        /// Returns true if the input is ignored. Escape key or speed-changing keys are still
        /// available as long as the graphical screen is enabled. (C: `!!opt_mode`)
        pub fn is_autoplay(&self) -> bool { self.mode != PlayMode }

        /// Returns true if the BGA is displayed. (C: `opt_bga < NO_BGA`)
        pub fn has_bga(&self) -> bool { self.bga != NoBga }

        /// Returns true if the BGA movie is enabled. (C: `opt_bga < BGA_BUT_NO_MOVIE`)
        pub fn has_movie(&self) -> bool { self.bga == BgaAndMovie }

        /// Returns true if the graphical screen is enabled.
        /// (C: `opt_mode < EXCLUSIVE_MODE || opt_bga < NO_BGA`)
        pub fn has_screen(&self) -> bool { !self.is_exclusive() || self.has_bga() }
    }

    //----------------------------------------------------------------------------------------------
    // bms utilities

    /// Parses a key specification from the options.
    pub fn key_spec(bms: &Bms, opts: &Options) -> Result<KeySpec,String> {
        use std::ascii::AsciiExt;
        use util::option::StrOption;

        let (leftkeys, rightkeys) =
            if opts.leftkeys.is_none() && opts.rightkeys.is_none() {
                let preset =
                    if opts.preset.is_none() &&
                       opts.bmspath[].to_ascii_lower()[].ends_with(".pms") {
                        Some("pms".to_string())
                    } else {
                        opts.preset.clone()
                    };
                match parser::preset_to_key_spec(bms, preset) {
                    Some(leftright) => leftright,
                    None => {
                        return Err(format!("Invalid preset name: {}",
                                           opts.preset.as_ref_slice_or("")));
                    }
                }
            } else {
                (opts.leftkeys.as_ref_slice_or("").to_string(),
                 opts.rightkeys.as_ref_slice_or("").to_string())
            };

        let mut keyspec = KeySpec { split: 0, order: Vec::new(),
                                    kinds: Vec::from_fn(NLANES, |_| None) };
        let parse_and_add = |keyspec: &mut KeySpec, keys: &str| -> Option<uint> {
            match parser::parse_key_spec(keys) {
                None => None,
                Some(ref left) if left.is_empty() => None,
                Some(left) => {
                    let mut err = false;
                    for &(lane,kind) in left.iter() {
                        if keyspec.kinds[*lane].is_some() { err = true; break; }
                        keyspec.order.push(lane);
                        keyspec.kinds[mut][*lane] = Some(kind);
                    }
                    if err {None} else {Some(left.len())}
                }
            }
        };

        if !leftkeys.is_empty() {
            match parse_and_add(&mut keyspec, leftkeys[]) {
                None => {
                    return Err(format!("Invalid key spec for left hand side: {}", leftkeys));
                }
                Some(nkeys) => { keyspec.split += nkeys; }
            }
        } else {
            return Err(format!("No key model is specified using -k or -K"));
        }
        if !rightkeys.is_empty() {
            match parse_and_add(&mut keyspec, rightkeys[]) {
                None => {
                    return Err(format!("Invalid key spec for right hand side: {}", rightkeys));
                }
                Some(nkeys) => { // no split panes except for #PLAYER 2
                    if bms.player != parser::COUPLE_PLAY { keyspec.split += nkeys; }
                }
            }
        }
        Ok(keyspec)
    }

    /// Applies given modifier to the game data. The target lanes of the modifier is determined
    /// from given key specification. This function should be called twice for the Couple Play,
    /// since 1P and 2P should be treated separately. (C: `shuffle_bms`)
    pub fn apply_modf<R: Rng>(bms: &mut Bms, modf: Modf, r: &mut R,
                              keyspec: &KeySpec, begin: uint, end: uint) {
        let mut lanes = Vec::new();
        for i in range(begin, end) {
            let lane = keyspec.order[i];
            let kind = keyspec.kinds[*lane];
            if modf == ShuffleExModf || modf == RandomExModf ||
                    kind.map_or(false, |kind| kind.counts_as_key()) {
                lanes.push(lane);
            }
        }

        match modf {
            MirrorModf => parser::apply_mirror_modf(bms, lanes[]),
            ShuffleModf | ShuffleExModf => parser::apply_shuffle_modf(bms, r, lanes[]),
            RandomModf | RandomExModf => parser::apply_random_modf(bms, r, lanes[])
        }
    }

    //----------------------------------------------------------------------------------------------
    // utilities

    /// Checks if the user pressed the escape key or the quit button. `atexit` is called before
    /// the program is terminated. (C: `check_exit`)
    pub fn check_exit(atexit: ||) {
        loop {
            match event::poll_event() {
                KeyEvent(event::EscapeKey,_,_,_) | QuitEvent => {
                    atexit();
                    ::util::exit(0);
                },
                NoEvent => { break; },
                _ => {}
            }
        }
    }

    /// Writes a line to the console without advancing to the next line. `s` should be short enough
    /// to be replaced (currently up to 72 bytes).
    pub fn update_line(s: &str) {
        let _ = write!(&mut std::io::stderr(), "\r{:72}\r{}", "", s);
    }

    /// A periodic timer for thresholding the rate of information display.
    pub struct Ticker {
        /// Minimal required milliseconds after the last display.
        pub interval: uint,
        /// The timestamp at the last display. It is a return value from `sdl::get_ticks` and
        /// measured in milliseconds. May be a `None` if the ticker is at the initial state or
        /// has been reset by `reset` method. (C: `lastinfo`)
        pub lastinfo: Option<uint>
    }

    impl Ticker {
        /// Returns a new ticker with a default display interval.
        pub fn new() -> Ticker {
            /// A reasonable interval for the console and graphic display.
            /// Currently set to about 21fps. (C: `INFO_INTERVAL`)
            const INFO_INTERVAL: uint = 47;
            Ticker { interval: INFO_INTERVAL, lastinfo: None }
        }

        /// Calls `f` only when required milliseconds have passed after the last display.
        /// `now` should be a return value from `sdl::get_ticks`.
        pub fn on_tick(&mut self, now: uint, f: ||) {
            if self.lastinfo.map_or(true, |t| now - t >= self.interval) {
                self.lastinfo = Some(now);
                f();
            }
        }

        /// Lets the next call to `on_tick` always call the callback.
        pub fn reset(&mut self) {
            self.lastinfo = None;
        }
    }

    //----------------------------------------------------------------------------------------------
    // initialization

    /// An internal sampling rate for SDL_mixer. Every chunk loaded is first converted to
    /// this sampling rate for the purpose of mixing.
    const SAMPLERATE: i32 = 44100;

    /// The number of bytes in the chunk converted to an internal sampling rate.
    const BYTESPERSEC: i32 = SAMPLERATE * 2 * 2; // stereo, 16 bits/sample

    /// Creates a small screen for BGAs (`BGAW` by `BGAH` pixels) if `exclusive` is set,
    /// or a full-sized screen (`SCREENW` by `SCREENH` pixels) otherwise. `fullscreen` is ignored
    /// when `exclusive` is set. (C: `init_ui` and `init_video`)
    pub fn init_video(exclusive: bool, fullscreen: bool) -> Surface {
        if !sdl::init([sdl::InitVideo]) {
            die!("SDL Initialization Failure: {}", sdl::get_error());
        }
        sdl_image::init([sdl_image::InitJPG, sdl_image::InitPNG]);

        let result =
            if exclusive {
                video::set_video_mode(BGAW as int, BGAH as int, 32,
                                      [video::SWSurface], [video::DoubleBuf])
            } else if !fullscreen {
                video::set_video_mode(SCREENW as int, SCREENH as int, 32,
                                      [video::SWSurface], [video::DoubleBuf])
            } else {
                video::set_video_mode(SCREENW as int, SCREENH as int, 32, [], [video::Fullscreen])
            };
        let screen =
            match result {
                Ok(screen) => screen,
                Err(err) => die!("SDL Video Initialization Failure: {}", err)
            };
        if !exclusive {
            sdl::mouse::set_cursor_visible(false);
        }
        sdl::wm::set_caption(::version()[], "");
        screen
    }

    /// Initializes SDL_mixer. (C: `init_ui`)
    pub fn init_audio() {
        if !sdl::init([sdl::InitAudio]) {
            die!("SDL Initialization Failure: {}", sdl::get_error());
        }
        //sdl_mixer::init([sdl_mixer::InitOGG, sdl_mixer::InitMP3]); // TODO
        if sdl_mixer::open(SAMPLERATE, audio::S16_AUDIO_FORMAT, audio::Stereo, 2048).is_err() {
            die!("SDL Mixer Initialization Failure");
        }
    }

    /// Initializes a joystick with given index.
    pub fn init_joystick(joyidx: uint) -> joy::Joystick {
        if !sdl::init([sdl::InitJoystick]) {
            die!("SDL Initialization Failure: {}", sdl::get_error());
        }
        unsafe {
            joy::ll::SDL_JoystickEventState(1); // TODO rust-sdl patch
        }
        match joy::Joystick::open(joyidx as int) {
            Ok(joy) => joy,
            Err(err) => die!("SDL Joystick Initialization Failure: {}", err)
        }
    }

    //----------------------------------------------------------------------------------------------
    // virtual input

    /// Actual input. Mapped to zero or more virtual inputs by input mapping.
    #[deriving(PartialEq,Eq)]
    enum Input {
        /// Keyboard input.
        KeyInput(event::Key),
        /// Joystick axis input.
        JoyAxisInput(uint),
        /// Joystick button input.
        JoyButtonInput(uint)
    }

    impl hash::Hash for Input {
        fn hash(&self, state: &mut hash::sip::SipState) {
            match *self {
                KeyInput(key) => { 0u8.hash(state); (key as uint).hash(state); }
                JoyAxisInput(axis) => { 1u8.hash(state); axis.hash(state); }
                JoyButtonInput(button) => { 2u8.hash(state); button.hash(state); }
            }
        }
    }

    /// Virtual input.
    #[deriving(PartialEq,Eq)]
    enum VirtualInput {
        /// Virtual input mapped to the lane.
        LaneInput(Lane),
        /// Speed down input (normally F3).
        SpeedDownInput,
        /// Speed up input (normally F4).
        SpeedUpInput
    }

    /**
     * State of virtual input elements. There are three states: neutral, and positive or negative.
     * There is no difference between positive and negative states (the naming is arbitrary)
     * except for that they are distinct.
     *
     * The states should really be one of pressed (non-neutral) or unpressed (neutral) states,
     * but we need two non-neutral states since the actual input device with continuous values
     * (e.g. joystick axes) can trigger the state transition *twice* without hitting the neutral
     * state. We solve this problem by making the transition from negative to positive (and vice
     * versa) temporarily hit the neutral state.
     */
    #[deriving(PartialEq,Eq)]
    pub enum InputState {
        /// Positive input state. Occurs when the button is pressed or the joystick axis is moved
        /// in the positive direction.
        Positive = 1,
        /// Neutral input state. Occurs when the button is not pressed or the joystick axis is moved
        /// back to the origin.
        Neutral = 0,
        /// Negative input state. Occurs when the joystick axis is moved in the negative direction.
        Negative = -1
    }

    impl VirtualInput {
        /// Returns true if the virtual input has a specified key kind in the key specification.
        pub fn active_in_key_spec(&self, kind: KeyKind, keyspec: &KeySpec) -> bool {
            match *self {
                LaneInput(Lane(lane)) => keyspec.kinds[lane] == Some(kind),
                SpeedDownInput | SpeedUpInput => true
            }
        }
    }

    /// An information about an environment variable for multiple keys.
    struct KeySet {
        envvar: &'static str,
        default: &'static str,
        mapping: &'static [(Option<KeyKind>, &'static [VirtualInput])],
    }

    /// A list of environment variables that set the mapping for multiple keys, and corresponding
    /// default values and the order of keys. (C: `envvars`)
    static KEYSETS: &'static [KeySet] = &[
        KeySet { envvar: "ANGOLMOIS_1P_KEYS",
                 default: "left shift%axis 3|z%button 3|s%button 6|x%button 2|d%button 7|\
                            c%button 1|f%button 4|v%axis 2|left alt",
                 mapping: &[(Some(parser::Scratch),   &[LaneInput(Lane(6))]),
                            (Some(parser::WhiteKey),  &[LaneInput(Lane(1))]),
                            (Some(parser::BlackKey),  &[LaneInput(Lane(2))]),
                            (Some(parser::WhiteKey),  &[LaneInput(Lane(3))]),
                            (Some(parser::BlackKey),  &[LaneInput(Lane(4))]),
                            (Some(parser::WhiteKey),  &[LaneInput(Lane(5))]),
                            (Some(parser::BlackKey),  &[LaneInput(Lane(8))]),
                            (Some(parser::WhiteKey),  &[LaneInput(Lane(9))]),
                            (Some(parser::FootPedal), &[LaneInput(Lane(7))])] },
        KeySet { envvar: "ANGOLMOIS_2P_KEYS",
                 default: "right alt|m|k|,|l|.|;|/|right shift",
                 mapping: &[(Some(parser::FootPedal), &[LaneInput(Lane(36+7))]),
                            (Some(parser::WhiteKey),  &[LaneInput(Lane(36+1))]),
                            (Some(parser::BlackKey),  &[LaneInput(Lane(36+2))]),
                            (Some(parser::WhiteKey),  &[LaneInput(Lane(36+3))]),
                            (Some(parser::BlackKey),  &[LaneInput(Lane(36+4))]),
                            (Some(parser::WhiteKey),  &[LaneInput(Lane(36+5))]),
                            (Some(parser::BlackKey),  &[LaneInput(Lane(36+8))]),
                            (Some(parser::WhiteKey),  &[LaneInput(Lane(36+9))]),
                            (Some(parser::Scratch),   &[LaneInput(Lane(36+6))])] },
        KeySet { envvar: "ANGOLMOIS_PMS_KEYS",
                 default: "z|s|x|d|c|f|v|g|b",
                 mapping: &[(Some(parser::Button1), &[LaneInput(Lane(1))]),
                            (Some(parser::Button2), &[LaneInput(Lane(2))]),
                            (Some(parser::Button3), &[LaneInput(Lane(3))]),
                            (Some(parser::Button4), &[LaneInput(Lane(4))]),
                            (Some(parser::Button5), &[LaneInput(Lane(5))]),
                            (Some(parser::Button4), &[LaneInput(Lane(8)), LaneInput(Lane(36+2))]),
                            (Some(parser::Button3), &[LaneInput(Lane(9)), LaneInput(Lane(36+3))]),
                            (Some(parser::Button2), &[LaneInput(Lane(6)), LaneInput(Lane(36+4))]),
                            (Some(parser::Button1), &[LaneInput(Lane(7)), LaneInput(Lane(36+5))])]
               },
        KeySet { envvar: "ANGOLMOIS_SPEED_KEYS",
                 default: "f3|f4",
                 mapping: &[(None, &[SpeedDownInput]),
                            (None, &[SpeedUpInput])] },
    ];

    /// An input mapping, i.e. a mapping from the actual input to the virtual input.
    pub type KeyMap = HashMap<Input,VirtualInput>;

    /// Reads an input mapping from the environment variables. (C: `read_keymap`)
    pub fn read_keymap(keyspec: &KeySpec, getenv: |&str| -> Option<String>) -> KeyMap {
        use std::ascii::{AsciiExt, OwnedAsciiExt};

        /// Finds an SDL virtual key with the given name. Matching is done case-insensitively.
        fn sdl_key_from_name(name: &str) -> Option<event::Key> {
            let name = name.to_ascii_lower();
            unsafe {
                let firstkey = 0u16;
                let lastkey = std::mem::transmute(event::LastKey);
                for keyidx in range(firstkey, lastkey) {
                    let key = std::mem::transmute(keyidx);
                    let keyname = event::get_key_name(key).into_ascii_lower();
                    if keyname == name { return Some(key); }
                }
            }
            None
        }

        /// Parses an `Input` value from the string. E.g. `"backspace"`, `"button 2"` or `"axis 0"`.
        fn parse_input(s: &str) -> Option<Input> {
            let mut idx = 0;
            let s = s.trim();
            if lex!(s; lit "button", ws, uint -> idx) {
                Some(JoyButtonInput(idx))
            } else if lex!(s; lit "axis", ws, uint -> idx) {
                Some(JoyAxisInput(idx))
            } else {
                sdl_key_from_name(s).map(|key| KeyInput(key))
            }
        }

        let mut map = HashMap::new();
        let add_mapping = |map: &mut KeyMap, kind: Option<KeyKind>,
                           input: Input, vinput: VirtualInput| {
            if kind.map_or(true, |kind| vinput.active_in_key_spec(kind, keyspec)) {
                map.insert(input, vinput);
            }
        };

        for &keyset in KEYSETS.iter() {
            let spec = getenv(keyset.envvar);
            let spec = spec.unwrap_or(keyset.default.to_string());

            let mut i = 0;
            for part in spec[].split('|') {
                let (kind, vinputs) = keyset.mapping[i];
                for s in part.split('%') {
                    match parse_input(s) {
                        Some(input) => {
                            for &vinput in vinputs.iter() {
                                add_mapping(&mut map, kind, input, vinput);
                            }
                        }
                        None => die!("Unknown key name in the environment \
                                      variable {}: {}", keyset.envvar, s)
                    }
                }

                i += 1;
                if i >= keyset.mapping.len() { break; }
            }
        }

        for &lane in keyspec.order.iter() {
            let key = Key(36 + *lane as int);
            let kind = keyspec.kinds[*lane].unwrap();
            let envvar = format!("ANGOLMOIS_{}{}_KEY", key, kind.to_char());
            for s in getenv(envvar[]).iter() {
                match parse_input(s[]) {
                    Some(input) => { add_mapping(&mut map, Some(kind), input, LaneInput(lane)); }
                    None => {
                        die!("Unknown key name in the environment variable {}: {}", envvar, *s);
                    }
                }
            }
        }

        map
    }

    //----------------------------------------------------------------------------------------------
    // resource management

    /// Alternative file extensions for sound resources. (C: `SOUND_EXTS`)
    static SOUND_EXTS: &'static [&'static str] = &[".WAV", ".OGG", ".MP3"];
    /// Alternative file extensions for image resources. (C: `IMAGE_EXTS`)
    static IMAGE_EXTS: &'static [&'static str] = &[".BMP", ".PNG", ".JPG", ".JPEG", ".GIF"];

    /// Returns a specified or implied resource directory from the BMS file.
    fn get_basedir(bms: &Bms, opts: &Options) -> Path {
        // TODO this logic assumes that #PATH_WAV is always interpreted as a native path, which
        // the C version doesn't assume. this difference barely makes the practical issue though.
        match bms.basepath {
            Some(ref basepath) => { Path::new(basepath[]) }
            None => {
                // Rust: it turns out that `Path("")` is always invalid. huh?
                let path = Path::new(opts.bmspath[]).dir_path();
                if path.components().count() == 0 {Path::new(".")} else {path}
            }
        }
    }

    /**
     * Resolves the specified resource path to the actual path if possible. May fail, but its
     * success doesn't guarantee that the resource should be read without a failure either.
     * (C: `resolve_relative_path`)
     *
     * The actual resolution is complicated by the fact that many BMSes assume the case-insensitive
     * matching on file names and the coexistence between WAV resources and MP3 resources while
     * keeping the same BMS file. Therefore Angolmois adopted the following resolution rules:
     *
     * 1. Both `/` and `\` are accepted as a directory separator.
     * 2. Path components including file names are matched case-insensitively. If there are multiple
     *    matches then any one can be used, even when a better match exists.
     * 3. If the initial match on the file name fails, and the file name does contain an extension,
     *    then a list of alternative extensions is applied with the same matching procedure.
     */
    fn resolve_relative_path(basedir: &Path, path: &str, exts: &[&str]) -> Option<Path> {
        use std::{str, io};
        use std::ascii::AsciiExt;
        use std::collections::hashmap::{Occupied, Vacant};
        use std::io::fs::PathExtensions;

        // `std::io::fs::readdir` is different from C's `dirent.h`, as it always reads
        // the whole list of entries (and `std::io::fs::Directories` is no different).
        // This causes a serious slowdown compared to the C version of Angolmois,
        // so we use a thread-local cache for `readdir` to avoid the performance penalty.
        local_data_key!(key_readdir_cache: HashMap<Path,Vec<Path>>);

        fn readdir_cache(path: Path, cb: |&[Path]|) {
            let mut cache = match key_readdir_cache.replace(None) {
                Some(cache) => cache,
                None => HashMap::new()
            };

            match cache.entry(path.clone()) {
                Occupied(entry) => {
                    cb(entry.get()[]);
                }
                Vacant(entry) => {
                    let files = io::fs::readdir(&path).ok().unwrap_or(Vec::new());
                    cb(entry.set(files)[mut]);
                }
            }

            key_readdir_cache.replace(Some(cache));
        }

        let mut parts = Vec::new();
        for part in path.split(|c: char| c == '/' || c == '\\') {
            if part.is_empty() { continue; }
            parts.push(part);
        }
        if parts.is_empty() { return None; }

        let mut cur = basedir.clone();
        let lastpart = parts.pop().unwrap();
        for part in parts.iter() {
            // early exit if the intermediate path does not exist or is not a directory
            if !cur.is_dir() { return None; }

            let part = part.to_ascii_upper();
            let mut found = false;
            readdir_cache(cur.clone(), |entries| {
                for next in entries.iter() {
                    let name = next.filename().and_then(str::from_utf8).map(|v| v.to_ascii_upper());
                    if name.as_ref().map_or(false, |name| *name == part) {
                        cur = next.clone();
                        found = true;
                        break;
                    }
                }
            });
            if !found { return None; }
        }

        if !cur.is_dir() { return None; }

        let lastpart = lastpart.to_ascii_upper();
        let mut ret = None;
        readdir_cache(cur, |entries| {
            for next in entries.iter() {
                let name = next.filename().and_then(str::from_utf8).map(|v| v.to_ascii_upper());
                let mut found = name.as_ref().map_or(false, |name| *name == lastpart);
                if !found && name.is_some() {
                    let name = name.unwrap();
                    match name[].rfind('.') {
                        Some(idx) => {
                            let namenoext = name[..idx];
                            for ext in exts.iter() {
                                if namenoext.to_string() + *ext == lastpart {
                                    found = true;
                                    break;
                                }
                            }
                        }
                        None => {} // does not try alternative extensions if there was no extension
                    }
                }
                if found {
                    ret = Some(next.clone());
                    break;
                }
            }
        });

        ret
    }

    /// Sound resource associated to `SoundRef`. It contains the actual SDL_mixer chunk that can be
    /// readily played. (C: the type of `sndres`)
    pub enum SoundResource {
        /// No sound resource is associated, or error occurred while loading.
        NoSound,
        /// Sound resource is associated.
        Sound(Chunk)
    }

    impl SoundResource {
        /// Returns the associated chunk if any.
        pub fn chunk<'r>(&'r self) -> Option<&'r Chunk> {
            match *self {
                NoSound => None,
                Sound(ref chunk) => Some(chunk)
            }
        }

        /// Returns the associated chunk if any.
        pub fn mut_chunk<'r>(&'r mut self) -> Option<&'r mut Chunk> {
            match *self {
                NoSound => None,
                Sound(ref mut chunk) => Some(chunk)
            }
        }

        /// Returns the length of associated sound chunk in seconds. This is used for determining
        /// the actual duration of the song in presence of key and background sounds, so it may
        /// return 0.0 if no sound is present.
        pub fn duration(&self) -> f64 {
            match *self {
                NoSound => 0.0,
                Sound(ref chunk) => {
                    let chunk = chunk.to_ll_chunk();
                    (unsafe {(*chunk).alen} as f64) / (BYTESPERSEC as f64)
                }
            }
        }
    }

    /// Loads a sound resource.
    fn load_sound(key: Key, path: &str, basedir: &Path) -> SoundResource {
        let res = match resolve_relative_path(basedir, path, SOUND_EXTS) {
            Some(fullpath) => Chunk::from_wav(&fullpath),
            None => Err(format!("not found"))
        };
        match res {
            Ok(res) => Sound(res),
            Err(_) => {
                warn!("failed to load sound \\#WAV{} ({})", key, path);
                NoSound
            }
        }
    }

    /// Image resource associated to `ImageRef`. It can be either a static image or a movie, and
    /// both contains an SDL surface that can be blitted to the screen. (C: the type of `imgres`)
    pub enum ImageResource {
        /// No image resource is associated, or error occurred while loading.
        NoImage,
        /// A static image is associated. The surface may have a transparency which is already
        /// handled by `load_image`.
        Image(Surface),
        /// A movie is associated. A playback starts when `start_movie` method is called, and stops
        /// when `stop_movie` is called. An associated surface is updated from the separate thread
        /// during the playback.
        Movie(Surface, MPEG)
    }

    impl ImageResource {
        /// Returns an associated surface if any.
        pub fn surface<'r>(&'r self) -> Option<&'r Surface> {
            match *self {
                NoImage => None,
                Image(ref surface) | Movie(ref surface,_) => Some(surface)
            }
        }

        /// Stops the movie playback if possible.
        pub fn stop_movie(&self) {
            match *self {
                NoImage | Image(_) => {}
                Movie(_,ref mpeg) => { mpeg.stop(); }
            }
        }

        /// Starts (or restarts, if the movie was already being played) the movie playback
        /// if possible.
        pub fn start_movie(&self) {
            match *self {
                NoImage | Image(_) => {}
                Movie(_,ref mpeg) => { mpeg.rewind(); mpeg.play(); }
            }
        }
    }

    /// Loads an image resource.
    fn load_image(key: Key, path: &str, opts: &Options, basedir: &Path) -> ImageResource {
        use std::ascii::AsciiExt;

        /// Converts a surface to the native display format, while preserving a transparency or
        /// setting a color key if required.
        fn to_display_format(surface: Surface) -> Result<Surface,String> {
            if unsafe {(*(*surface.raw).format).Amask} != 0 {
                let res = surface.display_format_alpha();
                match res {
                    Ok(ref surface) => {
                        surface.set_alpha([video::SrcAlpha, video::RLEAccel], 255);
                    }
                    _ => {}
                }
                res
            } else {
                let res = surface.display_format();
                match res {
                    Ok(ref surface) => {
                        surface.set_color_key([video::SrcColorKey, video::RLEAccel], RGB(0,0,0));
                    }
                    _ => {}
                }
                res
            }
        }

        if path.to_ascii_lower()[].ends_with(".mpg") {
            if opts.has_movie() {
                let res = match resolve_relative_path(basedir, path, []) {
                    Some(fullpath) => MPEG::from_path(&fullpath),
                    None => Err(format!("not found"))
                };
                match res {
                    Ok(movie) => {
                        let surface = gfx::new_surface(BGAW, BGAH);
                        movie.enable_video(true);
                        movie.set_loop(true);
                        movie.set_display(&surface);
                        return Movie(surface, movie);
                    }
                    Err(_) => { warn!("failed to load image \\#BMP{} ({})", key, path); }
                }
            }
        } else if opts.has_bga() {
            let res = match resolve_relative_path(basedir, path, IMAGE_EXTS) {
                Some(fullpath) => sdl_image::load(&fullpath).and_then(|surface| {
                    to_display_format(surface).and_then(|surface| Ok(Image(surface)))
                }),
                None => Err(format!("not found"))
            };
            match res {
                Ok(res) => { return res; },
                Err(_) => { warn!("failed to load image \\#BMP{} ({})", key, path); }
            }
        }
        NoImage
    }

    /// Applies the blit command to given list of image resources. (C: a part of `load_resource`)
    fn apply_blitcmd(imgres: &mut [ImageResource], bc: &BlitCmd) {
        use std::mem;

        let src = **bc.src as uint;
        let dst = **bc.dst as uint;
        if src == dst { return; }

        match imgres[src] {
            Image(..) => {}
            _ => { return; }
        }
        match imgres[dst] {
            Image(..) => {}
            NoImage => {
                let surface = gfx::new_surface(BGAW, BGAH);
                surface.fill(RGB(0, 0, 0));
                surface.set_color_key([video::SrcColorKey, video::RLEAccel], RGB(0, 0, 0));
                imgres[dst] = Image(surface);
            }
            _ => { return; }
        }

        // temporarily swap imgres[src], otherwise it will cause an error
        let savedorigin = mem::replace(&mut imgres[src], NoImage);
        {
            let origin = savedorigin.surface().unwrap();
            let target = imgres[dst].surface().unwrap();

            let x1 = cmp::max(bc.x1, 0);
            let y1 = cmp::max(bc.y1, 0);
            let x2 = cmp::min(bc.x2, bc.x1 + BGAW as int);
            let y2 = cmp::min(bc.y2, bc.y1 + BGAH as int);
            target.blit_area(origin, (x1,y1), (bc.dx,bc.dy), (x2-x1,y2-y1));
        }
        imgres[src] = savedorigin;
    }

    /// A list of image references displayed in BGA layers (henceforth the BGA state). Not all image
    /// referenced here is directly rendered, but the references themselves are kept.
    pub type BGAState = [Option<ImageRef>, ..NLAYERS];

    /// Returns the initial BGA state. Note that merely setting a particular layer doesn't start
    /// the movie playback; `poorbgafix` in `parser::parse` function handles it.
    pub fn initial_bga_state() -> BGAState {
        [None, None, None, Some(ImageRef(Key(0)))]
    }

    /// A trait for BGA state.
    trait BGAStateOps {
        /// Updates the BGA state. This method prepares given image resources for the next
        /// rendering, notably by starting and stopping the movie playback.
        fn update(&mut self, current: &BGAState, imgres: &[ImageResource]);
        /// Renders the image resources for the specified layers to the specified region of
        /// `screen`.
        fn render(&self, screen: &Surface, layers: &[BGALayer], imgres: &[ImageResource],
                  x: uint, y: uint);
    }

    impl BGAStateOps for BGAState {
        fn update(&mut self, current: &BGAState, imgres: &[ImageResource]) {
            for layer in range(0, NLAYERS) {
                // TODO this design can't handle the case that a BGA layer is updated to the same
                // image reference, which should rewind the movie playback. the original Angolmois
                // does handle it.
                if self[layer] != current[layer] {
                    for &iref in self[layer].iter() {
                        imgres[**iref as uint].stop_movie();
                    }
                    for &iref in current[layer].iter() {
                        imgres[**iref as uint].start_movie();
                    }
                }
            }
            *self = *current;
        }

        fn render(&self, screen: &Surface, layers: &[BGALayer], imgres: &[ImageResource],
                  x: uint, y: uint) {
            screen.fill_area((x,y), (256u,256u), RGB(0,0,0));
            for &layer in layers.iter() {
                for &iref in self[layer as uint].iter() {
                    for &surface in imgres[**iref as uint].surface().iter() {
                        screen.blit_area(surface, (0u,0u), (x,y), (256u,256u));
                    }
                }
            }
        }
    }

    //----------------------------------------------------------------------------------------------
    // loading

    /// Returns the interface string common to the graphical and textual loading screen.
    fn displayed_info(bms: &Bms, infos: &BmsInfo,
                      keyspec: &KeySpec) -> (String, String, String, String) {
        use util::option::StrOption;

        let meta = format!("Level {level} | BPM {bpm:.2}{hasbpmchange} | \
                            {nnotes} {nnotes_text} [{nkeys}KEY{haslongnote}]",
                           level = bms.playlevel, bpm = *bms.initbpm,
                           hasbpmchange = if infos.hasbpmchange {"?"} else {""},
                           nnotes = infos.nnotes as uint,
                           nnotes_text = if infos.nnotes == 1 {"note"} else {"notes"},
                           nkeys = keyspec.nkeys(),
                           haslongnote = if infos.haslongnote {"-LN"} else {""});
        let title = bms.title.as_ref_slice_or("").to_string();
        let genre = bms.genre.as_ref_slice_or("").to_string();
        let artist = bms.artist.as_ref_slice_or("").to_string();
        (meta, title, genre, artist)
    }

    /// Renders the graphical loading screen by blitting BMS #STAGEFILE image (if any) and showing
    /// the metadata. (C: `play_show_stagefile` when `opt_mode < EXCLUSIVE_MODE`)
    pub fn show_stagefile_screen(bms: &Bms, infos: &BmsInfo, keyspec: &KeySpec, opts: &Options,
                                 screen: &Surface, font: &Font) {
        let (meta, title, genre, artist) = displayed_info(bms, infos, keyspec);

        screen.with_pixels(|pixels| {
            font.print_string(pixels, SCREENW/2, SCREENH/2-16, 2, Centered, "loading bms file...",
                              Gradient::new(RGB(0x80,0x80,0x80), RGB(0x20,0x20,0x20)));
        });
        screen.flip();

        screen.with_pixels(|pixels| {
            for path in bms.stagefile.iter() {
                let basedir = get_basedir(bms, opts);
                for path in resolve_relative_path(&basedir, path[], IMAGE_EXTS).iter() {
                    match sdl_image::load(path).and_then(|s| s.display_format()) {
                        Ok(surface) => {
                            surface.with_pixels(|srcpixels| {
                                gfx::bicubic_interpolation(srcpixels, pixels);
                            });
                        }
                        Err(_) => {}
                    }
                }
            }

            if opts.showinfo {
                let bg = RGBA(0x10,0x10,0x10,0x40);
                let fg = Gradient::new(RGB(0xff,0xff,0xff), RGB(0x80,0x80,0x80));
                for i in range(0, SCREENW) {
                    for j in range(0, 42u) {
                        pixels.put_blended_pixel(i, j, bg);
                    }
                    for j in range(SCREENH-20, SCREENH) {
                        pixels.put_blended_pixel(i, j, bg);
                    }
                }
                font.print_string(pixels, 6, 4, 2, LeftAligned, title[], fg);
                font.print_string(pixels, SCREENW-8, 4, 1, RightAligned, genre[], fg);
                font.print_string(pixels, SCREENW-8, 20, 1, RightAligned, artist[], fg);
                font.print_string(pixels, 3, SCREENH-18, 1, LeftAligned, meta[], fg);
            }
        });

        screen.flip();
    }

    /// Renders the textual loading screen by printing the metadata.
    /// (C: `play_show_stagefile` when `opt_mode >= EXCLUSIVE_MODE`)
    pub fn show_stagefile_noscreen(bms: &Bms, infos: &BmsInfo, keyspec: &KeySpec, opts: &Options) {
        if opts.showinfo {
            let (meta, title, genre, artist) = displayed_info(bms, infos, keyspec);
            let _ = writeln!(&mut std::io::stderr(), "\
----------------------------------------------------------------------------------------------
Title:    {title}
Genre:    {genre}
Artist:   {artist}
{meta}
----------------------------------------------------------------------------------------------",
                title = title, genre = genre, artist = artist, meta = meta);
        }
    }

    /// Loads the image and sound resources and calls a callback whenever a new resource has been
    /// loaded. (C: `load_resource`)
    pub fn load_resource(bms: &Bms, opts: &Options,
                         callback: |Option<String>|) -> (Vec<SoundResource>, Vec<ImageResource>) {
        let basedir = get_basedir(bms, opts);

        let sndres: Vec<_> =
            bms.sndpath.iter().enumerate().map(|(i, path)| {
                match *path {
                    Some(ref path) => {
                        callback(Some(path.to_string()));
                        load_sound(Key(i as int), path[], &basedir)
                    },
                    None => NoSound
                }
            }).collect();
        let mut imgres: Vec<_> =
            bms.imgpath.iter().enumerate().map(|(i, path)| {
                match *path {
                    Some(ref path) => {
                        callback(Some(path.to_string()));
                        load_image(Key(i as int), path[], opts, &basedir)
                    },
                    None => NoImage
                }
            }).collect();

        for bc in bms.blitcmd.iter() {
            apply_blitcmd(imgres[mut], bc);
        }
        (sndres, imgres)
    }

    /// Saves a portion of the screen for the use in `graphic_update_status`.
    pub fn save_screen_for_loading(screen: &Surface) -> Surface {
        let saved_screen = gfx::new_surface(SCREENW, 20);
        saved_screen.blit_area(screen, (0u,SCREENH-20), (0u,0u), (SCREENW,20u));
        saved_screen
    }

    /// A callback template for `load_resource` with the graphical loading screen.
    /// (C: `resource_loaded`)
    pub fn graphic_update_status(path: Option<String>, screen: &Surface, saved_screen: &Surface,
                                 font: &Font, ticker: &mut Ticker, atexit: ||) {
        use std::mem;

        let mut path = path;
        ticker.on_tick(sdl::get_ticks(), || {
            let path = mem::replace(&mut path, None);
            let msg = path.unwrap_or("loading...".to_string());
            screen.blit_at(saved_screen, 0, (SCREENH-20) as i16);
            screen.with_pixels(|pixels| {
                font.print_string(pixels, SCREENW-3, SCREENH-18, 1, RightAligned, msg[],
                                  Gradient::new(RGB(0xc0,0xc0,0xc0), RGB(0x80,0x80,0x80)));
            });
            screen.flip();
        });
        check_exit(atexit);
    }

    /// A callback template for `load_resource` with the textual loading screen.
    /// (C: `resource_loaded`)
    pub fn text_update_status(path: Option<String>, ticker: &mut Ticker, atexit: ||) {
        use std::mem;

        let mut path = path;
        ticker.on_tick(sdl::get_ticks(), || {
            match mem::replace(&mut path, None) {
                Some(path) => {
                    use util::str::StrUtil;
                    let path = if path.len() < 63 {path[]} else {path[].slice_upto(0, 63)};
                    update_line(format!("Loading: {}", path)[]);
                }
                None => { update_line("Loading done."); }
            }
        });
        check_exit(atexit);
    }

    //----------------------------------------------------------------------------------------------
    // pointers

    /// A pointer to the object. A pointer is used to implement common operations, e.g. iterating
    /// until given position, or finding the closest object with given condition. A pointer can also
    /// be used like an object when it points to the valid object.
    pub struct Pointer {
        /// A BMS data holding objects.
        pub bms: Rc<Bms>,
        /// The current position. Can be the past-the-end value.
        pub pos: uint,
        /// The next position used by `next_*` methods, which are required to delay advancing `pos`
        /// by one step (so that the first iteration sees the current pointer yet to be updated).
        /// Therefore `next` is initially set to `None`, then each `next_*` call sets `next` to
        /// what `pos` needs to be after the next invocation.
        pub next: Option<uint>,
    }

    /// Returns true if two pointers share the common BMS data.
    fn has_same_bms(lhs: &Pointer, rhs: &Pointer) -> bool {
        lhs.bms.deref() as *const Bms == rhs.bms.deref() as *const Bms
    }

    impl PartialEq for Pointer {
        fn eq(&self, other: &Pointer) -> bool {
            has_same_bms(self, other) && self.pos == other.pos
        }
        fn ne(&self, other: &Pointer) -> bool {
            !has_same_bms(self, other) || self.pos != other.pos
        }
    }

    impl PartialOrd for Pointer {
        fn partial_cmp(&self, other: &Pointer) -> Option<Ordering> {
            assert!(has_same_bms(self, other));
            self.pos.partial_cmp(&other.pos)
        }
    }

    impl Clone for Pointer {
        fn clone(&self) -> Pointer {
            Pointer { bms: self.bms.clone(), pos: self.pos, next: None }
        }
    }

    impl ObjQueryOps for Pointer {
        fn is_visible(&self) -> bool { self.objs()[self.pos].is_visible() }
        fn is_invisible(&self) -> bool { self.objs()[self.pos].is_invisible() }
        fn is_lnstart(&self) -> bool { self.objs()[self.pos].is_lnstart() }
        fn is_lndone(&self) -> bool { self.objs()[self.pos].is_lndone() }
        fn is_ln(&self) -> bool { self.objs()[self.pos].is_ln() }
        fn is_bomb(&self) -> bool { self.objs()[self.pos].is_bomb() }
        fn is_soundable(&self) -> bool { self.objs()[self.pos].is_soundable() }
        fn is_gradable(&self) -> bool { self.objs()[self.pos].is_gradable() }
        fn is_renderable(&self) -> bool { self.objs()[self.pos].is_renderable() }
        fn is_object(&self) -> bool { self.objs()[self.pos].is_object() }
        fn is_bgm(&self) -> bool { self.objs()[self.pos].is_bgm() }
        fn is_setbga(&self) -> bool { self.objs()[self.pos].is_setbga() }
        fn is_setbpm(&self) -> bool { self.objs()[self.pos].is_setbpm() }
        fn is_stop(&self) -> bool { self.objs()[self.pos].is_stop() }

        fn object_lane(&self) -> Option<Lane> { self.objs()[self.pos].object_lane() }
        fn sounds(&self) -> Vec<SoundRef> { self.objs()[self.pos].sounds() }
        fn keydown_sound(&self) -> Option<SoundRef> { self.objs()[self.pos].keydown_sound() }
        fn keyup_sound(&self) -> Option<SoundRef> { self.objs()[self.pos].keyup_sound() }
        fn through_sound(&self) -> Option<SoundRef> { self.objs()[self.pos].through_sound() }
        fn images(&self) -> Vec<ImageRef> { self.objs()[self.pos].images() }
        fn through_damage(&self) -> Option<Damage> { self.objs()[self.pos].through_damage() }
    }

    impl Pointer {
        /// Returns a pointer pointing the first object in `bms`.
        pub fn new(bms: Rc<Bms>) -> Pointer {
            Pointer { bms: bms, pos: 0, next: None }
        }

        /// Returns a pointer pointing given object in `bms`.
        pub fn new_with_pos(bms: Rc<Bms>, pos: uint) -> Pointer {
            Pointer { bms: bms, pos: pos, next: None }
        }

        /// Returns a reference to the list of underlying objects.
        fn objs<'r>(&'r self) -> &'r [Obj] { self.bms.objs[] }

        /// Returns the time of pointed object.
        pub fn time(&self) -> f64 { self.objs()[self.pos].time }

        /// Returns the number of a measure containing the pointed object.
        pub fn measure(&self) -> int { self.objs()[self.pos].measure() }

        /// Returns the associated game data of pointed object.
        pub fn data(&self) -> ObjData { self.objs()[self.pos].data }

        /// Resets the internal iteration state.
        pub fn reset(&mut self) {
            self.next = None;
        }

        /// Seeks to the first object which time is past the limit, if any.
        pub fn seek_until(&mut self, limit: f64) {
            let objs = self.bms.objs[];
            let nobjs = objs.len();
            while self.pos < nobjs {
                if objs[self.pos].time >= limit { break; }
                self.pos += 1;
            }
            self.next = None;
        }

        /// Tries to advance to the next object which time is within the limit.
        /// Returns false if it's impossible.
        pub fn next_until(&mut self, limit: f64) -> bool {
            let objs = self.bms.objs[];
            match self.next {
                Some(next) => { self.pos = next; }
                None => {}
            }
            if self.pos < objs.len() && objs[self.pos].time < limit {
                self.next = Some(self.pos + 1);
                true
            } else {
                self.next = None;
                false
            }
        }

        /// Seeks to the object pointed by the other pointer.
        pub fn seek_to(&mut self, limit: &Pointer) {
            assert!(has_same_bms(self, limit));
            assert!(limit.pos <= self.bms.objs.len());
            self.pos = limit.pos;
            self.next = None;
        }

        /// Tries to advance to the next object which precedes the other pointer.
        /// Returns false if it's impossible.
        pub fn next_to(&mut self, limit: &Pointer) -> bool {
            assert!(has_same_bms(self, limit));
            match self.next {
                Some(next) => { self.pos = next; }
                None => {}
            }
            if self.pos >= limit.pos { return false; }
            self.next = Some(self.pos + 1);
            true
        }

        /// Seeks to the end of objects.
        pub fn seek_to_end(&mut self) {
            self.pos = self.bms.objs.len();
            self.next = None;
        }

        /// Tries to advance to the next object. Returns false if it's the end of objects.
        pub fn next_to_end(&mut self) -> bool {
            let objs = self.bms.objs[];
            match self.next {
                Some(next) => { self.pos = next; }
                None => {}
            }
            if self.pos >= objs.len() { return false; }
            self.next = Some(self.pos + 1);
            true
        }

        /// Finds the next object that satisfies given condition if any, without updating itself.
        pub fn find_next_of_type(&self, cond: |&Obj| -> bool) -> Option<Pointer> {
            let objs = self.bms.objs[];
            let nobjs = objs.len();
            let mut i = self.pos;
            while i < nobjs {
                if cond(&objs[i]) {
                    return Some(Pointer { bms: self.bms.clone(), pos: i, next: None });
                }
                i += 1;
            }
            None
        }

        /// Finds the previous object that satisfies given condition if any, without updating
        /// itself.
        pub fn find_previous_of_type(&self, cond: |&Obj| -> bool) -> Option<Pointer> {
            let objs = self.bms.objs[];
            let mut i = self.pos;
            while i > 0 {
                i -= 1;
                if cond(&objs[i]) {
                    return Some(Pointer { bms: self.bms.clone(), pos: i, next: None });
                }
            }
            None
        }

        /// Finds the closest object from the virtual time `base` that satisfies given condition
        /// if any. `base` should lie between the pointed object and the previous object.
        /// The proximity is measured in terms of virtual time, which can differ from actual time.
        pub fn find_closest_of_type(&self, base: f64, cond: |&Obj| -> bool) -> Option<Pointer> {
            let previous = self.find_previous_of_type(|obj| cond(obj));
            let next = self.find_next_of_type(|obj| cond(obj));
            match (previous, next) {
                (None, None) => None,
                (None, Some(next)) => Some(next),
                (Some(previous), None) => Some(previous),
                (Some(previous), Some(next)) =>
                    if num::abs(previous.time() - base) <
                       num::abs(next.time() - base) { Some(previous) }
                    else { Some(next) }
            }
        }
    }

    //----------------------------------------------------------------------------------------------
    // game play logics

    /// Grades. Angolmois performs the time-based grading as long as possible (it can go wrong when
    /// the object is near the discontinuity due to the current implementation strategy).
    #[deriving(PartialEq,Eq)]
    pub enum Grade {
        /**
         * Issued when the player did not input the object at all, the player was pressing the key
         * while a bomb passes through the corresponding lane, or failed to unpress the key within
         * the grading area for the end of LN. Resets the combo number, decreases the gauge
         * by severe amount (`MISS_DAMAGE` unless specified by the bomb) and displays the POOR BGA
         * for moments.
         *
         * Several games also use separate grading areas for empty lanes next to the object,
         * in order to avoid continuing the consecutive run ("combo") of acceptable grades by
         * just pressing every keys in the correct timing instead of pressing only lanes containing
         * objects. While this system is not a bad thing (and there are several BMS implementations
         * that use it), it is tricky to implement for the all situations. Angolmois currently
         * does not use this system due to the complexity.
         */
        MISS = 0,
        /// Issued when the player inputed the object and the normalized time difference (that is,
        /// the time difference multiplied by `Player::gradefactor`) between the input point and
        /// the object is between `GOOD_CUTOFF` and `BAD_CUTOFF` milliseconds. Resets the combo
        /// number, decreases the gauge by moderate amount (`BAD_DAMAGE`) and displays the POOR BGA
        /// for moments.
        BAD  = 1,
        /// Issued when the player inputed the object and the normalized time difference is between
        /// `GREAT_CUTOFF` and `GOOD_CUTOFF` milliseconds. Both the combo number and gauge is
        /// left unchanged.
        GOOD = 2,
        /// Issued when the player inputed the object and the normalized time difference is between
        /// `COOL_CUTOFF` and `GREAT_CUTOFF` milliseconds. The combo number is increased by one and
        /// the gauge is replenished by small amount.
        GREAT = 3,
        /// Issued when the player inputed the object and the normalized time difference is less
        /// than `COOL_CUTOFF` milliseconds. The combo number is increased by one and the gauge is
        /// replenished by large amount.
        COOL = 4,
    }

    /// Required time difference in milliseconds to get at least COOL grade.
    const COOL_CUTOFF: f64 = 14.4;
    /// Required time difference in milliseconds to get at least GREAT grade.
    const GREAT_CUTOFF: f64 = 48.0;
    /// Required time difference in milliseconds to get at least GOOD grade.
    const GOOD_CUTOFF: f64 = 84.0;
    /// Required time difference in milliseconds to get at least BAD grade.
    const BAD_CUTOFF: f64 = 144.0;

    /// The number of available grades.
    const NGRADES: uint = 5;

    /// The maximum (internal) value for the gauge.
    const MAXGAUGE: int = 512;
    /// A base score per exact input. Actual score can increase by the combo (up to 2x) or decrease
    /// by the larger time difference.
    const SCOREPERNOTE: f64 = 300.0;

    /// A damage due to the MISS grading. Only applied when the grading is not due to the bomb.
    const MISS_DAMAGE: Damage = GaugeDamage(0.059);
    /// A damage due to the BAD grading.
    const BAD_DAMAGE: Damage = GaugeDamage(0.030);

    /// Game play states independent to the display.
    pub struct Player {
        /// The game play options.
        pub opts: Options,
        /// The current BMS data.
        pub bms: Rc<Bms>,
        /// The derived BMS information.
        pub infos: BmsInfo,
        /// The length of BMS file in seconds as calculated by `bms_duration`. (C: `duration`)
        pub duration: f64,
        /// The key specification.
        pub keyspec: KeySpec,
        /// The input mapping.
        pub keymap: KeyMap,

        /// Set to true if the corresponding object in `bms.objs` had graded and should not be
        /// graded twice. Its length equals to that of `bms.objs`. (C: `nograding` field in
        /// `struct obj`)
        pub nograding: Vec<bool>,
        /// Sound resources. (C: `res` field in `sndres`)
        pub sndres: Vec<SoundResource>,
        /// A sound chunk used for beeps. It always plays on the channel #0. (C: `beep`)
        pub beep: Chunk,
        /// Last channels in which the corresponding sound in `sndres` was played.
        /// (C: `lastch` field in `sndres`)
        pub sndlastch: Vec<Option<uint>>,
        /// Indices to last sounds which the channel has played. For every `x`, if `sndlastch[x] ==
        /// Some(y)` then `sndlastchmap[y] == Some(x)` and vice versa. (C: `sndlastchmap`)
        pub lastchsnd: Vec<Option<uint>>,
        /// Currently active BGA layers. (C: `bga`)
        pub bga: BGAState,

        /// The chart expansion rate, or "play speed". One measure has the length of 400 pixels
        /// times the play speed, so higher play speed means that objects will fall much more
        /// quickly (hence the name). (C: `playspeed`)
        pub playspeed: f64,
        /// The play speed targeted for speed change if any. It is also the value displayed while
        /// the play speed is changing. (C: `targetspeed`)
        pub targetspeed: Option<f64>,
        /// The current BPM. Can be negative, in that case the chart will scroll backwards.
        /// (C: `bpm`)
        pub bpm: BPM,
        /// The timestamp at the last tick. It is a return value from `sdl::get_ticks` and measured
        /// in milliseconds. (C: `now`)
        pub now: uint,
        /// The timestamp at the first tick. (C: `origintime`)
        pub origintime: uint,
        /**
         * The timestamp at the last discontinuity that breaks a linear relationship between
         * the virtual time and actual time. (C: `starttime`) Currently the following are
         * considered a discontinuity:
         *
         * - `origintime`
         * - A change in BPM
         * - A change in scaling factor of measure
         * - A scroll stopper (in this case, `stoptime` is first updated and `starttime` is updated
         *   at the end of stop)
         */
        pub starttime: uint,
        /// The timestamp at the end of ongoing scroll stopper, if any. (C: `stoptime`)
        pub stoptime: Option<uint>,
        /// The virtual time at the last discontinuity. (C: `startoffset`)
        pub startoffset: f64,
        /// The current scaling factor of measure. (C: `startshorten`)
        pub startshorten: f64,

        /// The virtual time at the bottom of the visible chart. (C: `bottom`)
        pub bottom: f64,
        /// The virtual time at the grading line. Currently same as `bottom`. (C: `line`)
        pub line: f64,
        /// The virtual time at the top of the visible chart. (C: `top`)
        pub top: f64,
        /// A pointer to the first `Obj` after `bottom`. (C: `pfront`)
        pub pfront: Pointer,
        /// A pointer to the first `Obj` after `line`. (C: `pcur`)
        pub pcur: Pointer,
        /// A pointer to the first `Obj` that haven't escaped the grading area. It is possible that
        /// this `Obj` haven't reached the grading area either. (C: `pcheck`)
        pub pcheck: Pointer,
        /// Pointers to `Obj`s for the start of LN which grading is in progress. (C: `pthru`)
        //
        // Rust: this is intended to be `[Option<Pointer>, ..NLANES]` but a fixed-size vector cannot
        //       be cloned.
        pub pthru: Vec<Option<Pointer>>,

        /// The scale factor for grading area. The factor less than 1 causes the grading area
        /// shrink. (C: `gradefactor`)
        pub gradefactor: f64,
        /// (C: `grademode` and `gradetime`)
        pub lastgrade: Option<(Grade,uint)>,
        /// The numbers of each grades. (C: `scocnt`)
        pub gradecounts: [uint, ..NGRADES],
        /// The last combo number, i.e. the number of objects graded at least GREAT. GOOD doesn't
        /// cause the combo number reset; BAD and MISS do. (C: `scombo`)
        pub lastcombo: uint,
        /// The best combo number so far. If the player manages to get no BADs and MISSes, then
        /// the combo number should end up with the number of note and LN objects
        /// (`BMSInfo::nnotes`). (C: `smaxcombo`)
        pub bestcombo: uint,
        /// The current score. (C: `score`)
        pub score: uint,
        /// The current health gauge. Should be no larger than `MAXGAUGE`. This can go negative
        /// (not displayed directly), which will require players much more efforts to survive.
        /// (C: `gauge`)
        pub gauge: int,
        /// The health gauge required to survive at the end of the song. Note that the gaugex
        /// less than this value (or even zero) doesn't cause the instant game over;
        /// only `InstantDeath` value from `Damage` does. (C: `survival`)
        pub survival: int,

        /// The number of keyboard or joystick keys, mapped to each lane and and currently pressed.
        /// (C: `keypressed[0]`)
        pub keymultiplicity: [uint, ..NLANES],
        /// The state of joystick axes. (C: `keypressed[1]`)
        pub joystate: [InputState, ..NLANES],
    }

    /// A list of play speed marks. `SpeedUpInput` and `SpeedDownInput` changes the play speed to
    /// the next/previous nearest mark. (C: `speeds`)
    static SPEED_MARKS: &'static [f64] = &[0.1, 0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.5, 2.0, 2.5, 3.0,
        3.5, 4.0, 4.5, 5.0, 5.5, 6.0, 7.0, 8.0, 10.0, 15.0, 25.0, 40.0, 60.0, 99.0];

    /// Finds the next nearest play speed mark if any.
    fn next_speed_mark(current: f64) -> Option<f64> {
        let mut prev = None;
        for &speed in SPEED_MARKS.iter() {
            if speed < current - 0.001 {
                prev = Some(speed);
            } else {
                return prev;
            }
        }
        None
    }

    /// Finds the previous nearest play speed mark if any.
    fn previous_speed_mark(current: f64) -> Option<f64> {
        let mut next = None;
        for &speed in SPEED_MARKS.iter().rev() {
            if speed > current + 0.001 {
                next = Some(speed);
            } else {
                return next;
            }
        }
        None
    }

    /// Creates a beep sound played on the play speed change. (C: `create_beep`)
    fn create_beep() -> Chunk {
        let samples: Vec<i32> = Vec::from_fn(12000, // approx. 0.14 seconds
            // sawtooth wave at 3150 Hz, quadratic decay after 0.02 seconds.
            |i| { let i = i as i32; (i%28-14) * cmp::min(2000, (12000-i)*(12000-i)/50000) });
        unsafe {
            slice::raw::buf_as_slice(samples.as_ptr() as *const u8, samples.len() * 4, |samples| {
                sdl_mixer::Chunk::new(samples.to_vec(), 128)
            })
        }
    }

    impl Player {
        /// Creates a new player object. The player object owns other related structures, including
        /// the options, BMS file, key specification, input mapping and sound resources.
        pub fn new(opts: Options, bms: Bms, infos: BmsInfo, duration: f64, keyspec: KeySpec,
                   keymap: KeyMap, sndres: Vec<SoundResource>) -> Player {
            let now = sdl::get_ticks();
            let initplayspeed = opts.playspeed;
            let originoffset = infos.originoffset;
            let startshorten = bms.shorten(originoffset as int);
            let gradefactor = 1.5 - cmp::min(bms.rank, 5) as f64 * 0.25;
            let initialgauge = MAXGAUGE * 500 / 1000;
            let survival = MAXGAUGE * 293 / 1000;
            let initbpm = bms.initbpm;
            let nobjs = bms.objs.len();
            let nsounds = sndres.len();

            let bms = Rc::new(bms);
            let pfront = Pointer::new(bms.clone());
            let pcur = Pointer::new(bms.clone());
            let pcheck = Pointer::new(bms.clone());
            let mut player = Player {
                opts: opts, bms: bms, infos: infos, duration: duration,
                keyspec: keyspec, keymap: keymap,

                nograding: Vec::from_elem(nobjs, false), sndres: sndres, beep: create_beep(),
                sndlastch: Vec::from_elem(nsounds, None), lastchsnd: Vec::new(),
                bga: initial_bga_state(),

                playspeed: initplayspeed, targetspeed: None, bpm: initbpm, now: now,
                origintime: now, starttime: now, stoptime: None, startoffset: originoffset,
                startshorten: startshorten,

                bottom: originoffset, line: originoffset, top: originoffset,
                pfront: pfront, pcur: pcur, pcheck: pcheck, pthru: Vec::from_fn(NLANES, |_| None),

                gradefactor: gradefactor, lastgrade: None, gradecounts: [0, ..NGRADES],
                lastcombo: 0, bestcombo: 0, score: 0, gauge: initialgauge, survival: survival,

                keymultiplicity: [0, ..NLANES], joystate: [Neutral, ..NLANES],
            };

            player.allocate_more_channels(64);
            sdl_mixer::reserve_channels(1); // so that the beep won't be affected
            player
        }

        /// Returns true if the specified lane is being pressed, either by keyboard, joystick
        /// buttons or axes.
        pub fn key_pressed(&self, lane: Lane) -> bool {
            self.keymultiplicity[*lane] > 0 || self.joystate[*lane] != Neutral
        }

        /// Returns the play speed displayed. Can differ from the actual play speed
        /// (`self.playspeed`) when the play speed is changing.
        pub fn nominal_playspeed(&self) -> f64 {
            self.targetspeed.unwrap_or(self.playspeed)
        }

        /// Updates the score and associated statistics according to grading. `scoredelta` is
        /// an weight normalized to [0,1] that is calculated from the distance between the object
        /// and the input time, and `damage` is an optionally associated `Damage` value for bombs.
        /// May return true when `Damage` resulted in the instant death. (C: `update_grade`)
        pub fn update_grade(&mut self, grade: Grade, scoredelta: f64,
                            damage: Option<Damage>) -> bool {
            self.gradecounts[grade as uint] += 1;
            self.lastgrade = Some((grade, self.now));
            self.score += (scoredelta * SCOREPERNOTE *
                           (1.0 + (self.lastcombo as f64) /
                                  (self.infos.nnotes as f64))) as uint;

            match grade {
                MISS | BAD => { self.lastcombo = 0; }
                GOOD => {}
                GREAT | COOL => {
                    // at most 5/512(1%) recover when the combo is topped
                    let weight = if grade == GREAT {2} else {3};
                    let cmbbonus = cmp::min(self.lastcombo as int, 100) / 50;
                    self.lastcombo += 1;
                    self.gauge = cmp::min(self.gauge + weight + cmbbonus, MAXGAUGE);
                }
            }
            self.bestcombo = cmp::max(self.bestcombo, self.lastcombo);

            match damage {
                Some(GaugeDamage(ratio)) => {
                    self.gauge -= (MAXGAUGE as f64 * ratio) as int; true
                }
                Some(InstantDeath) => {
                    self.gauge = cmp::min(self.gauge, 0); false
                }
                None => true
            }
        }

        /// Same as `update_grade`, but the grade is calculated from the normalized difference
        /// between the object and input time in milliseconds. The normalized distance equals to
        /// the actual time difference when `gradefactor` is 1.0. (C: `update_grade(grade,
        /// scoredelta, 0)` where `grade` and `scoredelta` are pre-calculated from `dist`)
        pub fn update_grade_from_distance(&mut self, dist: f64) {
            let dist = num::abs(dist);
            let (grade, damage) = if      dist <  COOL_CUTOFF {(COOL,None)}
                                  else if dist < GREAT_CUTOFF {(GREAT,None)}
                                  else if dist <  GOOD_CUTOFF {(GOOD,None)}
                                  else if dist <   BAD_CUTOFF {(BAD,Some(BAD_DAMAGE))}
                                  else                        {(MISS,Some(MISS_DAMAGE))};
            let scoredelta = 1.0 - dist / BAD_CUTOFF;
            let scoredelta = if scoredelta < 0.0 {0.0} else {scoredelta};
            let keepgoing = self.update_grade(grade, scoredelta, damage);
            assert!(keepgoing);
        }

        /// Same as `update_grade`, but with the predetermined damage value. Always results in MISS
        /// grade. May return true when the damage resulted in the instant death.
        /// (C: `update_grade(0, 0, damage)`)
        pub fn update_grade_from_damage(&mut self, damage: Damage) -> bool {
            self.update_grade(MISS, 0.0, Some(damage))
        }

        /// Same as `update_grade`, but always results in MISS grade with the standard damage value.
        /// (C: `update_grade(0, 0, 0)`)
        pub fn update_grade_to_miss(&mut self) {
            let keepgoing = self.update_grade(MISS, 0.0, Some(MISS_DAMAGE));
            assert!(keepgoing);
        }

        /// Allocate more SDL_mixer channels without stopping already playing channels.
        /// (C: `allocate_more_channels`)
        pub fn allocate_more_channels(&mut self, howmany: uint) {
            let howmany = howmany as libc::c_int;
            let nchannels = sdl_mixer::allocate_channels(-1 as libc::c_int);
            let nchannels = sdl_mixer::allocate_channels(nchannels + howmany) as uint;
            if self.lastchsnd.len() <= nchannels {
                let ncopies = nchannels - self.lastchsnd.len() + 1;
                self.lastchsnd.grow(ncopies, None);
            }
        }

        /// Plays a given sound referenced by `sref`. `bgm` indicates that the sound is a BGM and
        /// should be played with the lower volume and should in the different channel group from
        /// key sounds. (C: `play_sound`)
        pub fn play_sound(&mut self, sref: SoundRef, bgm: bool) {
            let sref = **sref as uint;

            if self.sndres[sref].chunk().is_none() { return; }
            let lastch = self.sndlastch[sref].map(|ch| ch as libc::c_int);

            // try to play on the last channel if it is not occupied by other sounds (in this case
            // the last channel info is removed)
            let mut ch;
            loop {
                ch = self.sndres[mut][sref].mut_chunk().unwrap().play(lastch, 0);
                if ch >= 0 { break; }
                self.allocate_more_channels(32);
            }

            let group = if bgm {1} else {0};
            sdl_mixer::set_channel_volume(Some(ch), if bgm {96} else {128});
            sdl_mixer::group_channel(Some(ch), Some(group));

            let ch = ch as uint;
            for &idx in self.lastchsnd[ch].iter() {
                self.sndlastch[mut][idx] = None;
            }
            self.sndlastch[mut][sref] = Some(ch);
            self.lastchsnd[mut][ch] = Some(sref);
        }

        /// Plays a given sound if `sref` is not zero. This reflects the fact that an alphanumeric
        /// key `00` is normally a placeholder.
        pub fn play_sound_if_nonzero(&mut self, sref: SoundRef, bgm: bool) {
            if **sref > 0 { self.play_sound(sref, bgm); }
        }

        /// Plays a beep. The beep is always played in the channel 0, which is excluded from
        /// the uniform key sound and BGM management. (C: `Mix_PlayChannel(0, beep, 0)`)
        pub fn play_beep(&mut self) {
            self.beep.play(Some(0), 0);
        }

        /// Breaks a continuity at given virtual time.
        fn break_continuity(&mut self, at: f64) {
            assert!(at >= self.startoffset);
            self.starttime += (self.bpm.measure_to_msec(at - self.startoffset) *
                               self.startshorten) as uint;
            self.startoffset = at;
        }

        /// Updates the player state. (C: `play_process`)
        pub fn tick(&mut self) -> bool {
            // smoothly change the play speed
            if self.targetspeed.is_some() {
                let target = self.targetspeed.unwrap();
                let delta = target - self.playspeed;
                if num::abs(delta) < 0.001 {
                    self.playspeed = target;
                    self.targetspeed = None;
                } else {
                    self.playspeed += delta * 0.1;
                }
            }

            // process the ongoing scroll stopper if any
            self.now = sdl::get_ticks();
            self.bottom = match self.stoptime {
                Some(t) => {
                    if self.now >= t {
                        self.starttime = t;
                        self.stoptime = None;
                    }
                    self.startoffset
                }
                None => {
                    let msecdiff = (self.now - self.starttime) as f64;
                    let measurediff = self.bpm.msec_to_measure(msecdiff);
                    self.startoffset + measurediff / self.startshorten
                }
            };

            // process the measure scale factor change
            let bottommeasure = self.bottom.floor();
            let curshorten = self.bms.shorten(bottommeasure as int);
            if bottommeasure >= -1.0 && self.startshorten != curshorten {
                self.break_continuity(bottommeasure);
                self.startshorten = curshorten;
            }

            //self.line = self.bms.adjust_object_time(self.bottom, 0.03 / self.playspeed);
            self.line = self.bottom;
            self.top = self.bms.adjust_object_time(self.bottom, 1.25 / self.playspeed);
            let lineshorten = self.bms.shorten(self.line.floor() as int);

            // apply object-like effects while advancing to new `pcur`
            self.pfront.seek_until(self.bottom);
            let mut prevpcur = Pointer::new_with_pos(self.bms.clone(), self.pcur.pos);
            self.pcur.reset();
            while self.pcur.next_until(self.line) {
                let time = self.pcur.time();
                match self.pcur.data() {
                    BGM(sref) => {
                        self.play_sound_if_nonzero(sref, true);
                    }
                    SetBGA(layer, iref) => {
                        self.bga[layer as uint] = iref;
                    }
                    SetBPM(newbpm) => {
                        self.break_continuity(time);
                        self.bpm = newbpm;
                    }
                    Stop(duration) => {
                        let msecs = duration.to_msec(self.bpm);
                        let newstoptime = msecs as uint + self.now;
                        self.stoptime =
                            Some(self.stoptime.map_or(newstoptime,
                                                      |t| cmp::max(t, newstoptime)));
                        self.startoffset = time;
                    }
                    Visible(_,sref) | LNStart(_,sref) => {
                        if self.opts.is_autoplay() {
                            for &sref in sref.iter() {
                                self.play_sound_if_nonzero(sref, false);
                            }
                            self.update_grade_from_distance(0.0);
                        }
                    }
                    _ => {}
                }
            }

            // grade objects that have escaped the grading area
            if !self.opts.is_autoplay() {
                self.pcheck.reset();
                while self.pcheck.next_to(&self.pcur) {
                    let dist = self.bpm.measure_to_msec(self.line - self.pcheck.time()) *
                               self.bms.shorten(self.pcheck.measure()) * self.gradefactor;
                    if dist < BAD_CUTOFF { break; }

                    if !self.nograding[self.pcheck.pos] {
                        for &Lane(lane) in self.pcheck.object_lane().iter() {
                            let missable =
                                match self.pcheck.data() {
                                    Visible(..) | LNStart(..) => true,
                                    LNDone(..) => self.pthru[lane].is_some(),
                                    _ => false,
                                };
                            if missable {
                                self.update_grade_to_miss();
                                self.pthru[mut][lane] = None;
                            }
                        }
                    }
                }
            }

            // process inputs
            loop {
                // map to the virtual input. results in `vkey` (virtual key), `state` (input state)
                // and `continuous` (true if the input is not discrete and `Negative` input state
                // matters).
                let (key, state) = match event::poll_event() {
                    NoEvent => { break; }
                    QuitEvent | KeyEvent(event::EscapeKey,_,_,_) => { return false; }
                    KeyEvent(key,true,_,_) => (KeyInput(key), Positive),
                    KeyEvent(key,false,_,_) => (KeyInput(key), Neutral),
                    JoyButtonEvent(_which,button,true) =>
                        (JoyButtonInput(button as uint), Positive),
                    JoyButtonEvent(_which,button,false) =>
                        (JoyButtonInput(button as uint), Neutral),
                    JoyAxisEvent(_which,axis,delta) if delta > 3200 =>
                        (JoyAxisInput(axis as uint), Positive),
                    JoyAxisEvent(_which,axis,delta) if delta < -3200 =>
                        (JoyAxisInput(axis as uint), Negative),
                    JoyAxisEvent(_which,axis,_delta) =>
                        (JoyAxisInput(axis as uint), Neutral),
                    _ => { continue; }
                };
                let vkey = match self.keymap.find(&key) {
                    Some(&vkey) => vkey,
                    None => { continue; }
                };
                let continuous = match key {
                    KeyInput(..) | JoyButtonInput(..) => false,
                    JoyAxisInput(..) => true
                };

                if self.opts.is_exclusive() { continue; }

                // Returns true if the given lane is previously pressed and now unpressed.
                // When the virtual input is mapped to multiple actual inputs it can update
                // the internal state but still return false.
                let is_unpressed = |player: &mut Player, lane: Lane,
                                    continuous: bool, state: InputState| {
                    if state == Neutral || (continuous &&
                                            player.joystate[*lane] != state) {
                        if continuous {
                            player.joystate[*lane] = state; true
                        } else {
                            if player.keymultiplicity[*lane] > 0 {
                                player.keymultiplicity[*lane] -= 1;
                            }
                            (player.keymultiplicity[*lane] == 0)
                        }
                    } else {
                        false
                    }
                };

                // Returns true if the given lane is previously unpressed and now pressed.
                // When the virtual input is mapped to multiple actual inputs it can update
                // the internal state but still return false.
                let is_pressed = |player: &mut Player, lane: Lane,
                                  continuous: bool, state: InputState| {
                    if state != Neutral {
                        if continuous {
                            player.joystate[*lane] = state; true
                        } else {
                            player.keymultiplicity[*lane] += 1;
                            (player.keymultiplicity[*lane] == 1)
                        }
                    } else {
                        false
                    }
                };

                let process_unpress = |player: &mut Player, lane: Lane| {
                    // if LN grading is in progress and it is not within the threshold then
                    // MISS grade is issued
                    let nextlndone =
                        player.pthru[*lane].as_ref().and_then(|thru| {
                            thru.find_next_of_type(|obj| {
                                obj.object_lane() == Some(lane) &&
                                obj.is_lndone()
                            })
                        });
                    for p in nextlndone.iter() {
                        let delta = player.bpm.measure_to_msec(p.time() - player.line) *
                                    lineshorten * player.gradefactor;
                        if num::abs(delta) < BAD_CUTOFF {
                            player.nograding[mut][p.pos] = true;
                        } else {
                            player.update_grade_to_miss();
                        }
                    }
                    player.pthru[mut][*lane] = None;
                };

                let process_press = |player: &mut Player, lane: Lane| {
                    // plays the closest key sound
                    let soundable = player.pcur.find_closest_of_type(player.line, |obj| {
                        obj.object_lane() == Some(lane) && obj.is_soundable()
                    });
                    for p in soundable.iter() {
                        for &sref in p.sounds().iter() {
                            player.play_sound(sref, false);
                        }
                    }

                    // tries to grade the closest gradable object in
                    // the grading area
                    let gradable = player.pcur.find_closest_of_type(player.line, |obj| {
                        obj.object_lane() == Some(lane) && obj.is_gradable()
                    });
                    for p in gradable.iter() {
                        if p.pos >= player.pcheck.pos && !player.nograding[p.pos] &&
                                                         !p.is_lndone() {
                            let dist = player.bpm.measure_to_msec(p.time() - player.line) *
                                       lineshorten * player.gradefactor;
                            if num::abs(dist) < BAD_CUTOFF {
                                if p.is_lnstart() {
                                    player.pthru[mut][*lane] =
                                        Some(Pointer::new_with_pos(player.bms.clone(), p.pos));
                                }
                                player.nograding[mut][p.pos] = true;
                                player.update_grade_from_distance(dist);
                            }
                        }
                    }
                    true
                };

                match (vkey, state) {
                    (SpeedDownInput, Positive) | (SpeedDownInput, Negative) => {
                        let current = self.targetspeed.unwrap_or(self.playspeed);
                        for &newspeed in next_speed_mark(current).iter() {
                            self.targetspeed = Some(newspeed);
                            self.play_beep();
                        }
                    }
                    (SpeedUpInput, Positive) | (SpeedUpInput, Negative) => {
                        let current = self.targetspeed.unwrap_or(self.playspeed);
                        for &newspeed in previous_speed_mark(current).iter() {
                            self.targetspeed = Some(newspeed);
                            self.play_beep();
                        }
                    }
                    (LaneInput(lane), state) => {
                        if !self.opts.is_autoplay() {
                            if is_unpressed(self, lane, continuous, state) {
                                process_unpress(self, lane);
                            }
                            if is_pressed(self, lane, continuous, state) {
                                process_press(self, lane);
                            }
                        }
                    }
                    (_, _) => {}
                }

            }

            // process bombs
            if !self.opts.is_autoplay() {
                prevpcur.reset();
                while prevpcur.next_to(&self.pcur) {
                    match prevpcur.data() {
                        Bomb(lane,sref,damage) if self.key_pressed(lane) => {
                            // ongoing long note is not graded twice
                            self.pthru[mut][*lane] = None;
                            for &sref in sref.iter() {
                                self.play_sound(sref, false);
                            }
                            if !self.update_grade_from_damage(damage) {
                                // instant death
                                self.pcur.seek_to_end();
                                return false;
                            }
                        },
                        _ => {}
                    }
                }
            }

            // determines if we should keep playing
            if self.bottom > (self.bms.nmeasures + 1) as f64 {
                if self.opts.is_autoplay() {
                    sdl_mixer::num_playing(None) != sdl_mixer::num_playing(Some(0))
                } else {
                    sdl_mixer::newest_in_group(Some(1)).is_some()
                }
            } else if self.bottom < self.infos.originoffset {
                false // special casing the negative BPM
            } else {
                true
            }
        }
    }

    /// Display interface.
    pub trait Display {
        /// Renders the current information from `player` to the screen or console. Called after
        /// each call to `Player::tick`.
        fn render(&mut self, player: &Player);
        /// Shows the game play result from `player` to the screen or console. Called only once.
        fn show_result(&self, player: &Player);
    }

    //----------------------------------------------------------------------------------------------
    // graphic display

    /// An appearance for each lane. (C: `struct tkeykind` and `tkeyleft`)
    pub struct LaneStyle {
        /// The left position of the lane in the final screen. (C: `tkeyleft`)
        pub left: uint,
        /// The left position of the lane in the object sprite. (C: `spriteleft` field)
        pub spriteleft: uint,
        /// The left position of the lane in the bomb sprite. (C: `spritebombleft` field)
        pub spritebombleft: uint,
        /// The width of lane. (C: `width` field)
        pub width: uint,
        /// The base color of object. The actual `Gradient` for drawing is derived from this color.
        /// (C: `basecolor` field)
        pub basecolor: Color
    }

    impl LaneStyle {
        /// Constructs a new `LaneStyle` object from given key kind and the left or right position.
        /// (C: `tkeykinds`)
        pub fn from_kind(kind: KeyKind, pos: uint, right: bool) -> LaneStyle {
            let (spriteleft, spritebombleft, width, color) = match kind {
                parser::WhiteKey    => ( 25,   0, 25, RGB(0x80,0x80,0x80)),
                parser::WhiteKeyAlt => ( 50,   0, 25, RGB(0xf0,0xe0,0x80)),
                parser::BlackKey    => ( 75,   0, 25, RGB(0x80,0x80,0xff)),
                parser::Button1     => (130, 100, 30, RGB(0xe0,0xe0,0xe0)),
                parser::Button2     => (160, 100, 30, RGB(0xff,0xff,0x40)),
                parser::Button3     => (190, 100, 30, RGB(0x80,0xff,0x80)),
                parser::Button4     => (220, 100, 30, RGB(0x80,0x80,0xff)),
                parser::Button5     => (250, 100, 30, RGB(0xff,0x40,0x40)),
                parser::Scratch     => (320, 280, 40, RGB(0xff,0x80,0x80)),
                parser::FootPedal   => (360, 280, 40, RGB(0x80,0xff,0x80)),
            };
            let left = if right {pos - width} else {pos};
            LaneStyle { left: left, spriteleft: spriteleft, spritebombleft: spritebombleft,
                        width: width, basecolor: color }
        }

        /// Renders required object and bomb images to the sprite.
        pub fn render_to_sprite(&self, sprite: &Surface) {
            let left = self.spriteleft;
            let noteleft = self.spriteleft + SCREENW;
            let bombleft = self.spritebombleft + SCREENW;
            assert!(sprite.get_width() as uint >= cmp::max(noteleft, bombleft) + self.width);

            // render a background sprite (0 at top, <1 at bottom)
            let backcolor = Gradient { zero: RGB(0,0,0), one: self.basecolor };
            for i in range(140, SCREENH - 80) {
                sprite.fill_area((left, i), (self.width, 1u),
                                 backcolor.blend(i as int - 140, 1000));
            }

            // render note and bomb sprites (1/2 at middle, 1 at border)
            let denom = self.width as int;
            let notecolor = Gradient { zero: RGB(0xff,0xff,0xff), one: self.basecolor };
            let bombcolor = Gradient { zero: RGB(0,0,0),          one: RGB(0xc0,0,0) };
            for i in range(0, self.width / 2) {
                let num = (self.width - i) as int;
                sprite.fill_area((noteleft+i, 0u), (self.width-i*2, SCREENH),
                                 notecolor.blend(num, denom));
                sprite.fill_area((bombleft+i, 0u), (self.width-i*2, SCREENH),
                                 bombcolor.blend(num, denom));
            }
        }

        /// Renders the lane background to the screen from the sprite.
        pub fn render_back(&self, screen: &Surface, sprite: &Surface, pressed: bool) {
            screen.fill_area((self.left, 30u), (self.width, SCREENH-110), RGB(0,0,0));
            if pressed {
                screen.blit_area(sprite, (self.spriteleft, 140u), (self.left, 140u),
                                 (self.width, SCREENH-220));
            }
        }

        /// Renders an object to the screen from the sprite.
        pub fn render_note(&self, screen: &Surface, sprite: &Surface, top: uint, bottom: uint) {
            screen.blit_area(sprite, (self.spriteleft + SCREENW, 0u),
                             (self.left, top), (self.width, bottom - top));
        }

        /// Renders a bomb object to the screen from the sprite.
        pub fn render_bomb(&self, screen: &Surface, sprite: &Surface, top: uint, bottom: uint) {
            screen.blit_area(sprite, (self.spritebombleft + SCREENW, 0u),
                             (self.left, top), (self.width, bottom - top));
        }
    }

    /// Builds a list of `LaneStyle`s from the key specification.
    fn build_lane_styles(keyspec: &KeySpec) ->
                                    Result<(uint, Option<uint>, Vec<(Lane,LaneStyle)>), String> {
        let mut leftmost = 0;
        let mut rightmost = SCREENW;
        let mut styles = Vec::new();
        for &lane in keyspec.left_lanes().iter() {
            let kind = keyspec.kinds[*lane];
            assert!(kind.is_some());
            let kind = kind.unwrap();
            let style = LaneStyle::from_kind(kind, leftmost, false);
            styles.push((lane, style));
            leftmost += style.width + 1;
            if leftmost > SCREENW - 20 {
                return Err(format!("The screen can't hold that many lanes"));
            }
        }
        for &lane in keyspec.right_lanes().iter() {
            let kind = keyspec.kinds[*lane];
            assert!(kind.is_some());
            let kind = kind.unwrap();
            let style = LaneStyle::from_kind(kind, rightmost, true);
            styles.push((lane, style));
            if rightmost < leftmost + 40 {
                return Err(format!("The screen can't hold that many lanes"));
            }
            rightmost -= style.width + 1;
        }
        let mut rightmost = if rightmost == SCREENW {None} else {Some(rightmost)};

        // move lanes to the center if there are too small number of lanes
        let cutoff = 165;
        if leftmost < cutoff {
            for i in range(0, keyspec.split) {
                let (_lane, ref mut style) = styles[mut][i];
                style.left += (cutoff - leftmost) / 2;
            }
            leftmost = cutoff;
        }
        if rightmost.map_or(false, |x| x > SCREENW - cutoff) {
            for i in range(keyspec.split, styles.len()) {
                let (_lane, ref mut style) = styles[mut][i];
                style.left -= (rightmost.unwrap() - (SCREENW - cutoff)) / 2;
            }
            rightmost = Some(SCREENW - cutoff);
        }

        Ok((leftmost, rightmost, styles))
    }

    /// Creates a sprite. (C: sprite construction portion of `play_prepare`)
    fn create_sprite(opts: &Options, leftmost: uint, rightmost: Option<uint>,
                     styles: &[(Lane,LaneStyle)]) -> Surface {
        let sprite = gfx::new_surface(SCREENW + 400, SCREENH);
        let black = RGB(0,0,0);
        let gray = RGB(0x40,0x40,0x40); // gray used for separators

        // render notes and lane backgrounds
        for &(_lane,style) in styles.iter() {
            style.render_to_sprite(&sprite);
        }

        // render panels
        sprite.with_pixels(|pixels| {
            let topgrad = Gradient { zero: RGB(0x60,0x60,0x60), one: RGB(0xc0,0xc0,0xc0) };
            let botgrad = Gradient { zero: RGB(0x40,0x40,0x40), one: RGB(0xc0,0xc0,0xc0) };
            for j in range(-244i, 556) {
                for i in range(-10i, 20) {
                    let c = (i*2+j*3+750) % 2000;
                    pixels.put_pixel((j+244) as uint, (i+10) as uint,
                                     topgrad.blend(850 - num::abs(c-1000), 700));
                }
                for i in range(-20i, 60) {
                    let c = (i*3+j*2+750) % 2000;
                    let bottom = (SCREENH - 60) as int;
                    pixels.put_pixel((j+244) as uint, (i+bottom) as uint,
                                     botgrad.blend(850 - num::abs(c-1000), 700));
                }
            }
        });
        sprite.fill_area((10u, SCREENH-36), (leftmost, 1u), gray);

        // erase portions of panels left unused
        let leftgap = leftmost + 20;
        let rightgap = rightmost.map_or(SCREENW, |x| x - 20);
        let gapwidth = rightgap - leftgap;
        sprite.fill_area((leftgap, 0u), (gapwidth, 30u), black);
        sprite.fill_area((leftgap, SCREENH-80), (gapwidth, 80u), black);
        sprite.with_pixels(|pixels| {
            for i in range(0, 20u) {
                // Rust: this cannot be `uint` since `-1u` underflows!
                for j in iter::range_step(20i, 0, -1) {
                    let j = j as uint;
                    if i*i + j*j <= 400 { break; } // circled border
                    pixels.put_pixel(leftmost + j, 10 + i, black);
                    pixels.put_pixel(leftmost + j, (SCREENH-61) - i, black);
                    for &right in rightmost.iter() {
                        pixels.put_pixel((right-j) - 1, 10 + i, black);
                        pixels.put_pixel((right-j) - 1, (SCREENH-61) - i, black);
                    }
                }
            }
        });

        // draw the gauge bar if needed
        if !opts.is_autoplay() {
            sprite.fill_area((0u, SCREENH-16), (368u, 16u), gray);
            sprite.fill_area((4u, SCREENH-12), (360u, 8u), black);
        }

        sprite
    }

    /// Full-featured graphic display. Used for the normal game play and automatic play mode.
    pub struct GraphicDisplay {
        /// Sprite surface generated by `create_sprite`. (C: `sprite`)
        pub sprite: Surface,
        /// Display screen. (C: `screen`)
        pub screen: Surface,
        /// Bitmap font.
        pub font: Font,
        /// Image resources. (C: `imgres`)
        pub imgres: Vec<ImageResource>,

        /// The leftmost X coordinate of the area next to the lanes, that is, the total width of
        /// left-hand-side lanes. (C: `tpanel1`)
        pub leftmost: uint,
        /// The rightmost X coordinate of the area next to the lanes, that is, the screen width
        /// minus the total width of right-hand-side lanes if any. `None` indicates the absence of
        /// right-hand-side lanes. (C: `tpanel2`)
        pub rightmost: Option<uint>,
        /// The order and appearance of lanes. (C: `tkey` and `tkeyleft`)
        pub lanestyles: Vec<(Lane,LaneStyle)>,
        /// The left coordinate of the BGA. (C: `tbgax`)
        pub bgax: uint,
        /// The top coordinate of the BGA. (C: `tbgay`)
        pub bgay: uint,

        /// If not `None`, indicates that the POOR BGA should be displayed until this timestamp.
        /// (C: `poorlimit`)
        pub poorlimit: Option<uint>,
        /// If not `None`, indicates that the grading information should be displayed until
        /// this timestamp. (C: `gradetime`)
        pub gradelimit: Option<uint>,
        /// Currently known state of BGAs.
        pub lastbga: BGAState,
    }

    /// The list of grade names and corresponding color scheme. (C: `tgradestr` and `tgradecolor`)
    static GRADES: &'static [(&'static str,Gradient)] = &[
        ("MISS",  Gradient { zero: RGB(0xff,0xc0,0xc0), one: RGB(0xff,0x40,0x40) }),
        ("BAD",   Gradient { zero: RGB(0xff,0xc0,0xff), one: RGB(0xff,0x40,0xff) }),
        ("GOOD",  Gradient { zero: RGB(0xff,0xff,0xc0), one: RGB(0xff,0xff,0x40) }),
        ("GREAT", Gradient { zero: RGB(0xc0,0xff,0xc0), one: RGB(0x40,0xff,0x40) }),
        ("COOL",  Gradient { zero: RGB(0xc0,0xc0,0xff), one: RGB(0x40,0x40,0xff) }),
    ];

    impl GraphicDisplay {
        /// Creates a new graphic display from the options, key specification, pre-allocated
        /// (usually by `init_video`) screen, pre-created bitmap fonts and pre-loaded
        /// image resources. The last three are owned by the display, others are not
        /// (in fact, should be owned by `Player`).
        pub fn new(opts: &Options, keyspec: &KeySpec, screen: Surface, font: Font,
                   imgres: Vec<ImageResource>) -> Result<GraphicDisplay,String> {
            let (leftmost, rightmost, styles) = match build_lane_styles(keyspec) {
                Ok(styles) => styles,
                Err(err) => { return Err(err); }
            };
            let centerwidth = rightmost.unwrap_or(SCREENW) - leftmost;
            let bgax = leftmost + (centerwidth - BGAW) / 2;
            let bgay = (SCREENH - BGAH) / 2;
            let sprite = create_sprite(opts, leftmost, rightmost, styles[]);

            let display = GraphicDisplay {
                sprite: sprite, screen: screen, font: font, imgres: imgres,
                leftmost: leftmost, rightmost: rightmost,
                lanestyles: styles, bgax: bgax, bgay: bgay,
                poorlimit: None, gradelimit: None, lastbga: initial_bga_state(),
            };

            display.screen.fill(RGB(0,0,0));
            display.restore_panel();
            display.screen.flip();

            Ok(display)
        }

        /// Restores the panels by blitting upper and bottom panels to the screen.
        fn restore_panel(&self) {
            let screen = &self.screen;
            let sprite = &self.sprite;
            screen.blit_area(sprite, (0u,0u), (0u,0u), (SCREENW,30u));
            screen.blit_area(sprite, (0u,SCREENH-80), (0u,SCREENH-80), (SCREENW,80u));
        }
    }

    impl Display for GraphicDisplay {
        fn render(&mut self, player: &Player) {
            let screen = &self.screen;
            let sprite = &self.sprite;
            let font = &self.font;

            // update display states
            for &(grade,when) in player.lastgrade.iter() {
                if grade == MISS {
                    // switches to the normal BGA after 600ms
                    let minlimit = when + 600;
                    self.poorlimit = Some(self.poorlimit.map_or(minlimit,
                                                                |t| cmp::max(t, minlimit)));
                }
                // grade disappears after 700ms
                let minlimit = when + 700;
                self.gradelimit = Some(self.gradelimit.map_or(minlimit,
                                                              |t| cmp::max(t, minlimit)));
            }
            if self.poorlimit < Some(player.now) { self.poorlimit = None; }
            if self.gradelimit < Some(player.now) { self.gradelimit = None; }
            self.lastbga.update(&player.bga, self.imgres[]);

            // render BGAs (should render before the lanes since lanes can overlap with BGAs)
            if player.opts.has_bga() {
                static POOR_LAYERS: [BGALayer, ..1] = [PoorBGA];
                static NORM_LAYERS: [BGALayer, ..3] = [Layer1, Layer2, Layer3];
                let layers = if self.poorlimit.is_some() {POOR_LAYERS[]} else {NORM_LAYERS[]};
                self.lastbga.render(&self.screen, layers, self.imgres[], self.bgax, self.bgay);
            }

            // fill the lanes to the border color
            screen.fill_area((0u, 30u), (self.leftmost, SCREENH-110), RGB(0x40,0x40,0x40));
            for &rightmost in self.rightmost.iter() {
                screen.fill_area((rightmost, 30u), (SCREENH-rightmost, 490u), RGB(0x40,0x40,0x40));
            }
            for &(lane,style) in self.lanestyles.iter() {
                style.render_back(screen, sprite, player.key_pressed(lane));
            }

            // set the clip area to avoid drawing on the panels
            screen.set_clip_area((0u, 30u), (SCREENW, SCREENH-110));

            // render objects
            let time_to_y = |time| {
                let adjusted = player.bms.adjust_object_position(player.bottom, time);
                (SCREENH-70) - (400.0 * player.playspeed * adjusted) as uint
            };
            for &(lane,style) in self.lanestyles.iter() {
                let front = player.pfront.find_next_of_type(|obj| {
                    obj.object_lane() == Some(lane) && obj.is_renderable()
                });
                if front.is_none() { continue; }
                let front = front.unwrap();

                // LN starting before the bottom and ending after the top
                if front.time() > player.top && front.is_lndone() {
                    style.render_note(screen, sprite, 30, SCREENH - 80);
                } else {
                    let mut i = front.pos;
                    let mut nextbottom = None;
                    let nobjs = player.bms.objs.len();
                    let top = player.top;
                    while i < nobjs && player.bms.objs[i].time <= top {
                        let y = time_to_y(player.bms.objs[i].time);
                        match player.bms.objs[i].data {
                            LNStart(lane0,_) if lane0 == lane => {
                                assert!(nextbottom.is_none());
                                nextbottom = Some(y);
                            }
                            LNDone(lane0,_) if lane0 == lane => {
                                let bottom = SCREENH-80;
                                style.render_note(screen, sprite, y,
                                                  nextbottom.unwrap_or(bottom));
                                nextbottom = None;
                            }
                            Visible(lane0,_) if lane0 == lane => {
                                assert!(nextbottom.is_none());
                                style.render_note(screen, sprite, y-5, y);
                            }
                            Bomb(lane0,_,_) if lane0 == lane => {
                                assert!(nextbottom.is_none());
                                style.render_bomb(screen, sprite, y-5, y);
                            }
                            _ => {}
                        }
                        i += 1;
                    }

                    for &y in nextbottom.iter() {
                        style.render_note(screen, sprite, 30, y);
                    }
                }
            }

            // render measure bars
            for i in range(player.bottom.floor() as int, player.top.floor() as int + 1) {
                let y = time_to_y(i as f64);
                screen.fill_area((0u, y), (self.leftmost, 1u), RGB(0xc0,0xc0,0xc0));
                for &rightmost in self.rightmost.iter() {
                    screen.fill_area((rightmost, y), (800-rightmost, 1u), RGB(0xc0,0xc0,0xc0));
                }
            }

            // render grading text
            if self.gradelimit.is_some() && player.lastgrade.is_some() {
                let gradelimit = self.gradelimit.unwrap();
                let (lastgrade,_) = player.lastgrade.unwrap();
                let (gradename,gradecolor) = GRADES[lastgrade as uint];
                let delta = (cmp::max(gradelimit - player.now, 400) - 400) / 15;
                screen.with_pixels(|pixels| {
                    font.print_string(pixels, self.leftmost/2, SCREENH/2 - 40 - delta, 2,
                                      Centered, gradename, gradecolor);
                    if player.lastcombo > 1 {
                        font.print_string(pixels, self.leftmost/2, SCREENH/2 - 12 - delta, 1,
                                          Centered, format!("{} COMBO",
                                                            player.lastcombo)[],
                                          Gradient::new(RGB(0xff,0xff,0xff), RGB(0x80,0x80,0x80)));
                    }
                    if player.opts.is_autoplay() {
                        font.print_string(pixels, self.leftmost/2, SCREENH/2 + 2 - delta, 1,
                                          Centered, "(AUTO)",
                                          Gradient::new(RGB(0xc0,0xc0,0xc0), RGB(0x40,0x40,0x40)));
                    }
                });
            }

            screen.set_clip_rect(&screen.get_rect());

            self.restore_panel();

            // render panel
            let elapsed = (player.now - player.origintime) / 1000;
            let duration = player.duration as uint;
            let durationmsec = (player.duration * 1000.0) as uint;
            screen.with_pixels(|pixels| {
                let black = RGB(0,0,0);
                font.print_string(pixels, 10, 8, 1, LeftAligned,
                                  format!("SCORE {:07}", player.score)[], black);
                let nominalplayspeed = player.nominal_playspeed();
                font.print_string(pixels, 5, SCREENH-78, 2, LeftAligned,
                                  format!("{:4.1}x", nominalplayspeed)[], black);
                font.print_string(pixels, self.leftmost-94, SCREENH-35, 1, LeftAligned,
                                  format!("{:02}:{:02} / {:02}:{:02}",
                                          elapsed/60, elapsed%60,
                                          duration/60, duration%60)[], black);
                font.print_string(pixels, 95, SCREENH-62, 1, LeftAligned,
                                  format!("@{:9.4}", player.bottom)[], black);
                font.print_string(pixels, 95, SCREENH-78, 1, LeftAligned,
                                  format!("BPM {:6.2}", *player.bpm)[], black);
                let timetick = cmp::min(self.leftmost, (player.now - player.origintime) *
                                                       self.leftmost / durationmsec);
                font.print_glyph(pixels, 6 + timetick, SCREENH-52, 1,
                                 95, RGB(0x40,0x40,0x40)); // glyph #95: tick
            });

            // render gauge
            if !player.opts.is_autoplay() {
                // cycles four times per measure, [0,40)
                let cycle = (160.0 * player.startshorten * player.bottom).floor() % 40.0;
                let width = if player.gauge < 0 {0}
                            else {player.gauge * 400 / MAXGAUGE - (cycle as int)};
                let width = cmp::min(cmp::max(width, 5), 360);
                let color = if player.gauge >= player.survival {RGB(0xc0,0,0)}
                            else {RGB(0xc0 - ((cycle * 4.0) as u8), 0, 0)};
                screen.fill_area((4u, SCREENH-12), (width, 8u), color);
            }

            screen.flip();
        }

        fn show_result(&self, player: &Player) {
            if player.opts.is_autoplay() { return; }

            // check if the song reached the last gradable object (otherwise the game play was
            // terminated by the user)
            let nextgradable = player.pcur.find_next_of_type(|obj| obj.is_gradable());
            if nextgradable.is_some() { return; }

            if player.gauge >= player.survival {
                println!("*** CLEARED! ***\n\
                          COOL  {:4}    GREAT {:4}    GOOD  {:4}\n\
                          BAD   {:4}    MISS  {:4}    MAX COMBO {}\n\
                          SCORE {:07} (max {:07})",
                         player.gradecounts[4], player.gradecounts[3],
                         player.gradecounts[2], player.gradecounts[1],
                         player.gradecounts[0], player.bestcombo,
                         player.score, player.infos.maxscore);
            } else {
                println!("YOU FAILED!");
            }
        }
    }

    //----------------------------------------------------------------------------------------------
    // text display

    /// Text-only display. Used for the exclusive mode with BGA disabled.
    pub struct TextDisplay {
        /// Ticker used for printing to the console.
        pub ticker: Ticker
    }

    impl TextDisplay {
        /// Creates a new text-only display.
        pub fn new() -> TextDisplay {
            TextDisplay { ticker: Ticker::new() }
        }
    }

    impl Display for TextDisplay {
        fn render(&mut self, player: &Player) {
            if !player.opts.showinfo { return; }

            self.ticker.on_tick(player.now, || {
                let elapsed = (player.now - player.origintime) / 100;
                let duration = (player.duration * 10.0) as uint;
                update_line(format!("{:02}:{:02}.{} / {:02}:{:02}.{} (@{pos:9.4}) | \
                                     BPM {bpm:6.2} | {lastcombo} / {nnotes} notes",
                                    elapsed/600, elapsed/10%60, elapsed%10,
                                    duration/600, duration/10%60, duration%10,
                                    pos = player.bottom, bpm = *player.bpm,
                                    lastcombo = player.lastcombo,
                                    nnotes = player.infos.nnotes)[]);
            });
        }

        fn show_result(&self, _player: &Player) {
            update_line("");
        }
    }

    //----------------------------------------------------------------------------------------------
    // BGA-only display

    /// BGA-only display. Used for the exclusive mode with BGA enabled.
    pub struct BGAOnlyDisplay {
        /// The underlying text-only display (as the BGA-only display lacks the on-screen display).
        pub textdisplay: TextDisplay,
        /// Display screen. (C: `screen`)
        pub screen: Surface,
        /// Image resources. (C: `imgres`)
        pub imgres: Vec<ImageResource>,
        /// Currently known state of BGAs.
        pub lastbga: BGAState,
    }

    impl BGAOnlyDisplay {
        /// Creates a new BGA-only display from the pre-created screen (usually by `init_video`) and
        /// pre-loaded image resources.
        pub fn new(screen: Surface, imgres: Vec<ImageResource>) -> BGAOnlyDisplay {
            BGAOnlyDisplay { textdisplay: TextDisplay::new(), screen: screen,
                             imgres: imgres, lastbga: initial_bga_state() }
        }
    }

    impl Display for BGAOnlyDisplay {
        fn render(&mut self, player: &Player) {
            self.lastbga.update(&player.bga, self.imgres[]);

            let layers = &[Layer1, Layer2, Layer3];
            self.lastbga.render(&self.screen, layers, self.imgres[], 0, 0);
            self.screen.flip();

            self.textdisplay.render(player);
        }

        fn show_result(&self, player: &Player) {
            self.textdisplay.show_result(player);
        }
    }

    //----------------------------------------------------------------------------------------------

}

//==================================================================================================
// entry point

/// Parses the BMS file, initializes the display, shows the loading screen and runs the game play
/// loop. (C: `play`)
pub fn play(opts: player::Options) {
    use std::collections::HashMap;
    use sdl::get_ticks;
    use sdl::video::Surface;

    // parses the file and sanitizes it
    let mut r = std::rand::task_rng();
    let mut bms = match parser::parse_bms(opts.bmspath[], &mut r) {
        Ok(bms) => bms,
        Err(err) => die!("Couldn't load BMS file: {}", err)
    };
    parser::sanitize_bms(&mut bms);

    // parses the key specification and further sanitizes `bms` with it
    let keyspec = match player::key_spec(&bms, &opts) {
        Ok(keyspec) => keyspec,
        Err(err) => die!("{}", err)
    };
    parser::compact_bms(&mut bms, &keyspec);
    let infos = parser::analyze_bms(&bms);

    // applies the modifier if any
    for &modf in opts.modf.iter() {
        player::apply_modf(&mut bms, modf, &mut r, &keyspec, 0, keyspec.split);
        if keyspec.split < keyspec.order.len() {
            player::apply_modf(&mut bms, modf, &mut r, &keyspec,
                               keyspec.split, keyspec.order.len());
        }
    }

    // initialize SDL
    player::init_audio();
    for &joyidx in opts.joystick.iter() { player::init_joystick(joyidx); }

    // uncompress and populate the bitmap font.
    let mut font = gfx::Font::new();
    font.create_zoomed_font(1);
    font.create_zoomed_font(2);
    let font = font;

    // initialize the screen if required
    let mut screen = None;
    let keymap;
    if opts.has_screen() {
        screen = Some(player::init_video(opts.is_exclusive(), opts.fullscreen));
        // read the input mapping (dependent to the SDL initialization)
        keymap = player::read_keymap(&keyspec, std::os::getenv);
    } else {
        keymap = HashMap::new();
    }

    // XXX we don't really need the environment here
    fn update_line() { player::update_line("") }
    fn noop() {}
    let atexit = if opts.is_exclusive() {update_line} else {noop};

    let (sndres, imgres) = {
        // render the loading screen
        let ticker = std::cell::RefCell::new(player::Ticker::new());
        let mut saved_screen = None; // XXX should be in a trait actually
        let _ = saved_screen; // Rust: avoids incorrect warning. (#3796)
        let update_status;
        if !opts.is_exclusive() {
            let screen_: &Surface = screen.as_ref().unwrap();
            player::show_stagefile_screen(&bms, &infos, &keyspec, &opts, screen_, &font);
            if opts.showinfo {
                saved_screen = Some(player::save_screen_for_loading(screen_));
                update_status = |path| {
                    let screen: &Surface = screen.as_ref().unwrap();
                    let saved_screen: &Surface = saved_screen.as_ref().unwrap();
                    player::graphic_update_status(path, screen, saved_screen, &font,
                                                  ticker.borrow_mut().deref_mut(), || atexit())
                };
            } else {
                update_status = |_path| {};
            }
        } else if opts.showinfo {
            player::show_stagefile_noscreen(&bms, &infos, &keyspec, &opts);
            update_status = |path| {
                player::text_update_status(path, ticker.borrow_mut().deref_mut(), || atexit())
            };
        } else {
            update_status = |_path| {};
        }

        // wait for resources
        let start = get_ticks() + 3000;
        let (sndres, imgres) =
            player::load_resource(&bms, &opts, |msg| update_status(msg));
        if opts.showinfo {
            ticker.borrow_mut().reset(); // force update
            update_status(None);
        }
        while get_ticks() < start { player::check_exit(|| atexit()); }

        (sndres, imgres)
    };

    // create the player and transfer ownership of other resources to it
    let duration = parser::bms_duration(&bms, infos.originoffset,
                                        |sref| sndres[**sref as uint].duration());
    let mut player = player::Player::new(opts, bms, infos, duration, keyspec, keymap, sndres);

    // create the display and runs the actual game play loop
    let mut display = match screen {
        Some(screen) => {
            if player.opts.is_exclusive() {
                box player::BGAOnlyDisplay::new(screen, imgres) as Box<player::Display>
            } else {
                let display_ = player::GraphicDisplay::new(&player.opts, &player.keyspec,
                                                           screen, font, imgres);
                match display_ {
                    Ok(display) => box display as Box<player::Display>,
                    Err(err) => die!("{}", err)
                }
            }
        },
        None => box player::TextDisplay::new() as Box<player::Display>
    };
    while player.tick() {
        display.render(&player);
    }
    display.show_result(&player);

    // remove all channels before sound resources are deallocated.
    // halting alone is not sufficient due to rust-sdl's bug.
    sdl_mixer::allocate_channels(0);

    // it's done!
    atexit();
}

/// Prints the usage. (C: `usage`)
pub fn usage() {
    let _ = write!(&mut std::io::stderr(), "\
{} -- the simple BMS player
http://mearie.org/projects/angolmois/
https://github.com/lifthrasiir/angolmois-rust/

Usage: {} <options> <path>
  Accepts any BMS, BME, BML or PMS file.
  Resources should be in the same directory as the BMS file.

Options:
  -h, --help              This help
  -V, --version           Shows the version
  -a X.X, --speed X.X     Sets the initial play speed (default: 1.0x)
  -1, .., -9              Same as '-a 1.0', .., '-a 9.0'
  -v, --autoplay          Enables AUTO PLAY (viewer) mode
  -x, --exclusive         Enables exclusive (BGA and sound only) mode
  -X, --sound-only        Enables sound only mode, equivalent to -xB
  --fullscreen            Enables the fullscreen mode (default)
  -w, --no-fullscreen     Disables the fullscreen mode
  --info                  Shows a brief information about the song (default)
  -q, --no-info           Do not show an information about the song
  -m, --mirror            Uses a mirror modifier
  -s, --shuffle           Uses a shuffle modifier
  -S, --shuffle-ex        Uses a shuffle modifier, even for scratches
  -r, --random            Uses a random modifier
  -R, --random-ex         Uses a random modifier, even for scratches
  -k NAME, --preset NAME  Forces a use of given key preset (default: bms)
  -K LEFT RIGHT, --key-spec LEFT RIGHT
                          Sets a custom key specification (see the manual)
  --bga                   Loads and shows the BGA (default)
  -B, --no-bga            Do not load and show the BGA
  -M, --no-movie          Do not load and show the BGA movie
  -j N, --joystick N      Enable the joystick with index N (normally 0)

Environment Variables:
  ANGOLMOIS_1P_KEYS=<scratch>|<key 1>|<2>|<3>|<4>|<5>|<6>|<7>|<pedal>
  ANGOLMOIS_2P_KEYS=<pedal>|<key 1>|<2>|<3>|<4>|<5>|<6>|<7>|<scratch>
  ANGOLMOIS_PMS_KEYS=<key 1>|<2>|<3>|<4>|<5>|<6>|<7>|<8>|<9>
  ANGOLMOIS_SPEED_KEYS=<speed down>|<speed up>
  ANGOLMOIS_XXy_KEY=<keys for channel XX and channel kind y>
    Sets keys used for game play. Use either SDL key names or joystick names
    like 'button N' or 'axis N' can be used. Separate multiple keys by '%'.
    See the manual for more information.

", version(), exename());
    util::exit(1);
}

/// The entry point. Parses the command line options and delegates other things to `play`.
/// (C: `main`)
pub fn main() {
    use player;
    use std::collections::HashMap;

    let longargs = vec!(
        ("--help", 'h'), ("--version", 'V'), ("--speed", 'a'),
        ("--autoplay", 'v'), ("--exclusive", 'x'), ("--sound-only", 'X'),
        ("--windowed", 'w'), ("--no-fullscreen", 'w'),
        ("--fullscreen", ' '), ("--info", ' '), ("--no-info", 'q'),
        ("--mirror", 'm'), ("--shuffle", 's'), ("--shuffle-ex", 'S'),
        ("--random", 'r'), ("--random-ex", 'R'), ("--preset", 'k'),
        ("--key-spec", 'K'), ("--bga", ' '), ("--no-bga", 'B'),
        ("--movie", ' '), ("--no-movie", 'M'), ("--joystick", 'j')
    ).into_iter().collect::<HashMap<&str,char>>();

    let args = std::os::args();
    let nargs = args.len();

    let mut bmspath = None;
    let mut mode = player::PlayMode;
    let mut modf = None;
    let mut bga = player::BgaAndMovie;
    let mut showinfo = true;
    let mut fullscreen = true;
    let mut joystick = None;
    let mut preset = None;
    let mut leftkeys = None;
    let mut rightkeys = None;
    let mut playspeed = 1.0;

    let mut i = 1;
    while i < nargs {
        let arg = args[i][];
        if !arg.starts_with("-") {
            if bmspath.is_none() {
                bmspath = Some(arg.to_string());
            }
        } else if arg == "--" {
            i += 1;
            if bmspath.is_none() && i < nargs {
                bmspath = Some(arg.to_string());
            }
            break;
        } else {
            let shortargs =
                if arg.starts_with("--") {
                    match longargs.find(&arg) {
                        Some(&c) => c.to_string(),
                        None => die!("Invalid option: {}", arg)
                    }
                } else {
                    arg[1..].to_string()
                };
            let nshortargs = shortargs.len();

            let mut inside = true;
            for (j, c) in shortargs[].chars().enumerate() {
                // Reads the argument of the option. Option string should be consumed first.
                macro_rules! fetch_arg(
                    ($opt:expr) => ({
                        let off = if inside {j+1} else {j};
                        let nextarg =
                            if inside && off < nshortargs {
                                // remaining portion of `args[i]` is an argument
                                shortargs[off..]
                            } else {
                                // `args[i+1]` is an argument as a whole
                                i += 1;
                                if i < nargs {
                                    args[i][]
                                } else {
                                    die!("No argument to the option -{}", $opt);
                                }
                            };
                        inside = false;
                        nextarg
                    })
                )

                match c {
                    'h' => { usage(); }
                    'V' => { println!("{}", version()); return; }
                    'v' => { mode = player::AutoPlayMode; }
                    'x' => { mode = player::ExclusiveMode; }
                    'X' => { mode = player::ExclusiveMode; bga = player::NoBga; }
                    'w' => { fullscreen = false; }
                    'q' => { showinfo = false; }
                    'm' => { modf = Some(player::MirrorModf); }
                    's' => { modf = Some(player::ShuffleModf); }
                    'S' => { modf = Some(player::ShuffleExModf); }
                    'r' => { modf = Some(player::RandomModf); }
                    'R' => { modf = Some(player::RandomExModf); }
                    'k' => { preset = Some(fetch_arg!('k').to_string()); }
                    'K' => { leftkeys = Some(fetch_arg!('K').to_string());
                             rightkeys = Some(fetch_arg!('K').to_string()); }
                    'a' => {
                        match from_str::<f64>(fetch_arg!('a')) {
                            Some(speed) if speed > 0.0 => {
                                playspeed = if speed < 0.1 {0.1}
                                            else if speed > 99.0 {99.0}
                                            else {speed};
                            }
                            _ => die!("Invalid argument to option -a")
                        }
                    }
                    'B' => { bga = player::NoBga; }
                    'M' => { bga = player::BgaButNoMovie; }
                    'j' => {
                        match from_str::<uint>(fetch_arg!('j')) {
                            Some(n) => { joystick = Some(n); }
                            _ => die!("Invalid argument to option -j")
                        }
                    }
                    ' ' => {} // for ignored long options
                    '1'...'9' => { playspeed = c.to_digit(10).unwrap() as f64; }
                    _ => die!("Invalid option: -{}", c)
                }
                if !inside { break; }
            }
        }
        i += 1;
    }

    // shows a file dialog if the path to the BMS file is missing and the system supports it
    if bmspath.is_none() {
        bmspath = util::get_path_from_dialog();
    }

    match bmspath {
        None => { usage(); }
        Some(bmspath) => {
            play(player::Options {
                bmspath: bmspath, mode: mode, modf: modf, bga: bga,
                showinfo: showinfo, fullscreen: fullscreen, joystick: joystick,
                preset: preset, leftkeys: leftkeys, rightkeys: rightkeys, playspeed: playspeed
            });
        }
    }
}
