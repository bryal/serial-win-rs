// The MIT License (MIT)
//
// Copyright (c) 2015 Johan Johansson
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

#![feature(collections)]
#![cfg_attr(test, feature(step_by))]

#[macro_use]
extern crate bitflags;
extern crate libc;
pub use ffi::*;

use libc::consts::os::extra::*;
use libc::funcs::extra::kernel32;
use libc::{ c_void, c_int, HANDLE };
use std::{ ptr, mem, io };
use std::io::{ Error, ErrorKind };
use std::cell::RefCell;

mod ffi;

fn system_to_io_err(operation: &'static str, error_code: c_int) -> io::Error {
	use std::io::ErrorKind::*;

	let (error_kind, message) = match error_code {
		ERROR_ACCESS_DENIED => (AlreadyExists, "Access denied. Resource might be busy"),
		ERROR_FILE_NOT_FOUND => (NotFound, "Serial port not found"),
		ERROR_INVALID_USER_BUFFER => (InvalidInput, "Supplied buffer is invalid"),
		ERROR_NOT_ENOUGH_MEMORY => (Other, "Too many I/O requests, not enough memory"),
		ERROR_OPERATION_ABORTED => (Interrupted, "Operation was canceled"),
		ERROR_INVALID_HANDLE => (InvalidInput, "Communications handle is invalid"),
		_ => (Other, "unmatched error"),
	};

	Error::new(error_kind,
		format!(r#"Operation `{}` failed with code 0x{:x} and message "{}""#,
			operation, error_code, message))
}

/// A serial connection
pub struct Connection {
	// Pointer to the serial connection
	comm_handle: RefCell<HANDLE>
}
impl Connection {
	/// Open a new connection via port `port` with baud rate `baud_rate`
	pub fn new(port: &str, baud_rate: u32) -> io::Result<Connection> {
		let (comm_handle, err) = unsafe {
			let mut port_u16: Vec<_> = port.utf16_units().collect();
			port_u16.push(0);
			(
				kernel32::CreateFileW(port_u16.as_ptr(),
					GENERIC_READ | GENERIC_WRITE,
					0,
					ptr::null_mut(),
					OPEN_EXISTING,
					libc::FILE_ATTRIBUTE_NORMAL,
					ptr::null_mut()),
				kernel32::GetLastError() as c_int
			)
		};

		if comm_handle == INVALID_HANDLE_VALUE {
			Err(system_to_io_err("Open port", err))
		} else {
			let mut conn = Connection{ comm_handle: RefCell::new(comm_handle) };

			conn.comm_state()
				.map(|mut dcb| {
					dcb.set_dtr_control(DTR_CONTROL::ENABLE);
					dcb
				})
				.and_then(|dcb| conn.set_comm_state(dcb))
				.and_then(|_| conn.set_baud_rate(baud_rate))
				.and_then(|_| conn.set_byte_size(8))
				.and_then(|_| conn.set_stop_bits(ONESTOPBIT))
				.and_then(|_| conn.set_parity(NOPARITY))
				.and_then(|_| {
					unsafe {
						PurgeComm(*conn.comm_handle.borrow_mut(), PURGE_RXCLEAR | PURGE_TXCLEAR);
					}
					conn.set_timeout(40)
				})
				.map(|_| conn)					
		}
	}

	/// Retrieve the current control settings for this communications device
	fn comm_state(&self) -> io::Result<DCB> {
		let mut dcb = unsafe { mem::zeroed() };
		let (succeded, err) = unsafe { (
			GetCommState(*self.comm_handle.borrow_mut(), &mut dcb) != 0,
			kernel32::GetLastError() as i32
		)};

		if succeded {
			Ok(dcb)
		} else {
			Err(system_to_io_err("GetCommState", err))
		}
	}

	/// Configures this communications device according to specifications in a device-control block,
	/// `dcb`.
	fn set_comm_state(&mut self, mut dcb: DCB) -> io::Result<()> {
		let (succeded, err) = unsafe { (
			SetCommState(*self.comm_handle.borrow_mut(), &mut dcb) != 0,
			kernel32::GetLastError() as i32
		)};

		if succeded {
			Ok(())
		} else {
			Err(system_to_io_err("SetCommState", err))
		}
	}

	/// Set interval and total timeouts to `timeout_ms`
	pub fn set_timeout(&mut self, timeout_ms: u32) -> io::Result<()> {
		let (succeded, err) = unsafe { (
			SetCommTimeouts(*self.comm_handle.borrow_mut(), &mut COMMTIMEOUTS{
				ReadIntervalTimeout: timeout_ms,
				ReadTotalTimeoutMultiplier: timeout_ms,
				ReadTotalTimeoutConstant: timeout_ms,
				WriteTotalTimeoutMultiplier: timeout_ms,
				WriteTotalTimeoutConstant: timeout_ms,
			}) != 0,
			kernel32::GetLastError() as i32
		)};

		if succeded {
			Ok(())
		} else {
			Err(system_to_io_err("SetCommTimeouts", err))
		}
	}

	pub fn baud_rate(&self) -> io::Result<u32> {
		self.comm_state().map(|dcb| dcb.BaudRate)
	}

	pub fn set_baud_rate(&mut self, baud_rate: u32) -> io::Result<()> {
		self.comm_state().and_then(|dcb| self.set_comm_state(DCB{ BaudRate: baud_rate, ..dcb }))
	}

	pub fn byte_size(&self) -> io::Result<u8> {
		self.comm_state().map(|dcb| dcb.ByteSize)
	}

	pub fn set_byte_size(&mut self, byte_size: u8) -> io::Result<()> {
		self.comm_state().and_then(|dcb| self.set_comm_state(DCB{ ByteSize: byte_size, ..dcb }))
	}

	pub fn parity(&self) -> io::Result<u8> {
		self.comm_state().map(|dcb| dcb.Parity)
	}

	pub fn set_parity(&mut self, parity: u8) -> io::Result<()> {
		self.comm_state().and_then(|dcb| self.set_comm_state(DCB{ Parity: parity, ..dcb }))
	}

	pub fn stop_bits(&self) -> io::Result<u8> {
		self.comm_state().map(|dcb| dcb.StopBits)
	}

	pub fn set_stop_bits(&mut self, stop_bits: u8) -> io::Result<()> {
		self.comm_state().and_then(|dcb| self.set_comm_state(DCB{ StopBits: stop_bits, ..dcb }))
	}

	/// Read into `buf` until `delim` is encountered. Return n.o. bytes read on success,
	/// and an IO error on failure.
	pub fn read_until(&mut self, delim: u8, buf: &mut Vec<u8>) -> io::Result<usize> {
		use std::io::Read;

		let mut byte = [0_u8];
		loop {
			match self.read(&mut byte) {
				Err(e) => return Err(e),
				Ok(0) => { println!("read 0 bytes"); break },
				Ok(_) => (),
			};

			let buf_len = buf.len();
			if byte[0] == delim {
				return Ok(buf_len);
			}

			if buf_len == buf.capacity() {
				buf.reserve(buf_len);
			}
			buf.push(byte[0])
		}

		// Delimiter was not found before end of stream or timeout
		Err(Error::new(ErrorKind::Other, "Delimiter was not found"))
	}

	/// Read until newline. Return n.o. bytes read on success
	pub fn read_line(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
		self.read_until('\n' as u8, buf)
	}
}
impl io::Read for Connection {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		if buf.len() == 0 {
			return Ok(0)
		}

		let mut n_bytes_read = 0;
		let (succeded, err) = unsafe { (
			kernel32::ReadFile(*self.comm_handle.borrow_mut(),
				buf.as_mut_ptr() as *mut c_void,
				buf.len() as u32,
				&mut n_bytes_read,
				ptr::null_mut()) != 0,
			kernel32::GetLastError() as c_int
		)};

		if succeded {
			if n_bytes_read == 0 {
				Err(Error::new(ErrorKind::TimedOut, "Operation timed out"))
			} else {
				Ok(n_bytes_read as usize)
			}
		} else {
			Err(system_to_io_err("read", err))
		}
	}
}
impl io::Write for Connection {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		let mut n_bytes_written = 0;

		let (succeded, err) = unsafe { (
			kernel32::WriteFile(*self.comm_handle.borrow_mut(),
				mem::transmute(buf.as_ptr()),
				buf.len() as u32,
				&mut n_bytes_written,
				ptr::null_mut()) != 0,
			kernel32::GetLastError() as c_int
		) };

		if succeded {
			Ok(n_bytes_written as usize)
		} else {
			Err(system_to_io_err("write", err))
		}
	}

	fn flush(&mut self) -> io::Result<()> {
		let (succeded, err) = unsafe { (
			kernel32::FlushFileBuffers(*self.comm_handle.borrow_mut()) != 0,
			kernel32::GetLastError() as c_int
		)};

		if succeded {
			Ok(())
		} else {
			Err(system_to_io_err("flush", err))
		}
	}
}
impl Drop for Connection {
	fn drop(&mut self) {
		let e = unsafe { kernel32::CloseHandle(*self.comm_handle.borrow_mut()) };
		if e == 0 {
			panic!("Drop of Connection failed. CloseHandle gave error 0x{:x}", e)
		}
	}
}
unsafe impl Send for Connection { }

// Some tests requires correct code to be running on connected serial device

// // Should work with any simple echo code on the arduino
// #[test]
// fn echo_test() {
// 	use std::io::Write;
// 	use std::thread;

// 	let port = "COM8";
// 	let baud_rate = 9600_u32;
// 	let mut connection = Connection::new(port, baud_rate).unwrap();
// 	thread::sleep_ms(2000);

// 	for i in 0..20 {
// 		let test_str = format!("One two {}\n", i);

// 		let n_bytes_written = connection.write(test_str.as_bytes());

// 		let mut buffer = Vec::with_capacity(20);

// 		let n_bytes_read = connection.read_line(&mut buffer);
// 		let read_string = String::from_utf8(buffer).unwrap();

// 		println!("Bytes written: {:?}, Bytes read: {:?}, String read: {}",
// 			n_bytes_written, n_bytes_read, read_string);
// 	}
// }

// Colorswirl. Works with arduino running LEDstream
#[test]
fn colorswirl_test() {
	use std::thread;
	use std::io::Write;

	let port = "COM8";
	let baud_rate = 115_200;
	let mut connection = Connection::new(port, baud_rate).unwrap();

	thread::sleep_ms(2000);

	let n_leds = 32;
	let pixel_size = 3;
	let header_size = 6;
	let mut buffer: Vec<u8> = (0..(header_size + n_leds * pixel_size)).map(|_| 0).collect();

	// A special header / magic word is expected by the corresponding LED streaming code 
	// running on the Arduino. This only needs to be initialized once because the number of  
	// LEDs remains constant:
	buffer[0] = 'A' as u8;                    // Magic word
	buffer[1] = 'd' as u8;
	buffer[2] = 'a' as u8;
	buffer[3] = ((n_leds - 1) >> 8) as u8;    // LED count high byte
	buffer[4] = ((n_leds - 1) & 0xff) as u8;  // LED count low byte
	buffer[5] = buffer[3] ^ buffer[4] ^ 0x55; // Checksum
	
	let mut main_sin = 0.0_f32;
	let mut main_hue = 0_u16;

	for _ in 0..1_000 {
		let mut internal_sin = main_sin;
		let mut internal_hue = main_hue;

		let (mut r, mut g, mut b): (u8, u8, u8);
		// Start at position 6, after the LED header/magic word
		for i in (6..buffer.len()).step_by(3) {
			// Fixed-point hue-to-RGB conversion.  'internal_hue' is an integer in the
			// range of 0 to 1535, where 0 = red, 256 = yellow, 512 = green, etc.
			// The high byte (0-5) corresponds to the sextant within the color
			// wheel, while the low byte (0-255) is the fractional part between
			// the primary/secondary colors.
			let pri_sec_frac = (internal_hue & 255) as u8;
			match (internal_hue >> 8) % 6 {
				0 => {
					r = 255;
					g = pri_sec_frac;
					b = 0;
				}, 1 => {
					r = 255 - pri_sec_frac;
					g = 255;
					b = 0;
				}, 2 => {
					r = 0;
					g = 255;
					b = pri_sec_frac;
				}, 3 => {
					r = 0;
					g = 255 - pri_sec_frac;
					b = 255;
				}, 4 => {
					r = pri_sec_frac;
					g = 0;
					b = 255;
				}, _ => {
					r = 255;
					g = 0;
					b = 255 - pri_sec_frac;
				}
			}

			// Resulting hue is multiplied by brightness in the range of 0 to 255
			// (0 = off, 255 = brightest).  Gamma corrrection (the 'powf' function
			// here) adjusts the brightness to be more perceptually linear.
			let brightness = (0.5 + internal_sin.sin() * 0.5).powf(2.8);
			buffer[i]     = (r as f32 * brightness) as u8;
			buffer[i + 1] = (g as f32 * brightness) as u8;
			buffer[i + 2] = (b as f32 * brightness) as u8;

			// Each pixel is slightly offset in both hue and brightness
			internal_hue += 40;
			internal_sin += 0.3;
		}

		// Slowly rotate hue and brightness in opposite directions
		main_hue = (main_hue + 4) % 1536;
		main_sin -= 0.03;

		// Issue color data to LEDs
		connection.write(&buffer[..]).unwrap();
		connection.flush().unwrap(); // Not necessary, just testing

		// The arduino can't handle it if we go too fast
		thread::sleep_ms(2);
	}
}