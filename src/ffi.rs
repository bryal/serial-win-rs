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

#![allow(non_snake_case, non_camel_case_types, dead_code, non_upper_case_globals)]

use libc::{ c_int, c_char, LPOVERLAPPED, HANDLE, DWORD, WORD, BOOL, BYTE, SECURITY_ATTRIBUTES };

pub const ERROR_INVALID_USER_BUFFER: c_int = 1784;
pub const ERROR_NOT_ENOUGH_MEMORY: c_int = 8;

bitflags!{
	#[repr(C)]
	flags DCBFlags: WORD {
		const DCBFBinary = 0x0001,
		const DCBFParity = 0x0002,
		const DCBFOutxCtsFlow = 0x0004,
		const DCBFOutxDsrFlow = 0x0008,
		const DCBFDtrControl_lo = 0x0010,
		const DCBFDtrControl_hi = 0x0020,
		const DCBFDsrSensitivity = 0x0040,
		const DCBFTXContinueOnXoff = 0x0080,
		const DCBFOutX = 0x0100,
		const DCBFInX = 0x0200,
		const DCBFErrorChar = 0x0400,
		const DCBFNull = 0x0800,
		const DCBFRtsControl_lo = 0x1000,
		const DCBFRtsControl_hi = 0x2000,
		const DCBFAbortOnError = 0x4000,
		const DCBFDummy = 0x8000,
	}
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct DCB {
	pub DCBlength: DWORD,
	pub BaudRate: DWORD,
	pub flags: DCBFlags,
	pub fDummy: WORD,
	pub wReserved: WORD,
	pub XonLim: WORD,
	pub XoffLim: WORD,
	pub ByteSize: BYTE,
	pub Parity: BYTE,
	pub StopBits: BYTE,
	pub XonChar: c_char,
	pub XoffChar: c_char,
	pub ErrorChar: c_char,
	pub EofChar: c_char,
	pub EvtChar: c_char,
	pub wReserved1: WORD,
}

impl DCB {
	pub fn set_dtr_control(&mut self, control: DTR_CONTROL) {
		match control {
			DTR_CONTROL::DISABLE => self.flags.remove(DCBFDtrControl_lo | DCBFDtrControl_hi),
			DTR_CONTROL::ENABLE => {
				self.flags.remove(DCBFDtrControl_hi);
				self.flags.insert(DCBFDtrControl_lo)
			},
			DTR_CONTROL::HANDSHAKE => self.flags.insert(DCBFDtrControl_lo | DCBFDtrControl_hi),
		}
	}
}

#[derive(Debug, Clone)]
pub enum Parity {
	NO = 0,
	ODD = 1,
	EVEN = 2,
	MARK = 3,
	SPACE = 4,
}

#[derive(Debug, Clone)]
pub enum StopBits {
	ONE = 0,
	ONE5 = 1,
	TWO = 2,
}

#[derive(Debug, Clone)]
pub enum DTR_CONTROL {
	DISABLE,
	ENABLE,
	HANDSHAKE,
}

bitflags!{
	#[repr(C)]
	flags CommEventFlags: DWORD {
		const EV_BREAK = 0x0040,
		const EV_CTS = 0x0008,
		const EV_DSR = 0x0010,
		const EV_ERR = 0x0080,
		const EV_RING = 0x0100,
		const EV_RLSD = 0x0020,
		const EV_RXCHAR = 0x0001,
		const EV_RXFLAG = 0x0002,
		const EV_TXEMPTY = 0x0004,
	}
}

bitflags!{
	#[repr(C)]
	flags PurgeFlags: DWORD {
		const PURGE_TXABORT = 0x0001,
		const PURGE_RXABORT = 0x0002,
		const PURGE_TXCLEAR = 0x0004,
		const PURGE_RXCLEAR = 0x0008,
	}
}

#[repr(C)]
pub struct COMMTIMEOUTS {
	pub ReadIntervalTimeout: DWORD,
	pub ReadTotalTimeoutMultiplier: DWORD,
	pub ReadTotalTimeoutConstant: DWORD,
	pub WriteTotalTimeoutMultiplier: DWORD,
	pub WriteTotalTimeoutConstant: DWORD,
}

#[link(name = "kernel32")]
extern "system" {
	pub fn PurgeComm(file_handle: HANDLE, flags: PurgeFlags) -> BOOL;
	pub fn GetCommState(file_handle: HANDLE, dcb: *mut DCB) -> BOOL;
	pub fn SetCommState(file_handle: HANDLE, dcb: *mut DCB) -> BOOL;
	pub fn SetCommMask(file_handle: HANDLE, event_mask: CommEventFlags) -> BOOL;
	pub fn WaitCommEvent(file_handle: HANDLE, event_mask: *mut CommEventFlags,
		overlapped: LPOVERLAPPED) -> BOOL;
	pub fn SetCommTimeouts(file_handle: HANDLE, comm_timeouts: *mut COMMTIMEOUTS) -> BOOL;
	pub fn CreateFileA(lpFileName: *const c_char, dwDesiredAccess: DWORD, dwShareMode: DWORD,
		lpSecurityAttributes: *mut SECURITY_ATTRIBUTES, dwCreationDisposition: DWORD,
		dwFlagsAndAttributes: DWORD, hTemplateFile: HANDLE) -> HANDLE;
}
