#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
use core::panic::PanicInfo;

use spleen_font::FONT_8X16;
use limine::BaseRevision;
use limine::framebuffer::Framebuffer;
use limine::request::{
    FramebufferRequest, RequestsEndMarker, RequestsStartMarker, StackSizeRequest,
};

const STACK_SIZE_BYTES: u64 = 512 * 1024;

#[used]
#[unsafe(link_section = ".requests_start")]
static REQUESTS_START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests")]
static STACK_SIZE_REQUEST: StackSizeRequest = StackSizeRequest::new().with_size(STACK_SIZE_BYTES);

#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[unsafe(link_section = ".requests_end")]
static REQUESTS_END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    draw_banner(b"Hello, world!\nLimine + Rust kernel is alive.");
    halt_forever();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let _ = info;
    draw_banner(b"Kernel panic\nSystem halted.");
    halt_forever();
}

fn draw_banner(message: &[u8]) {
    let response = FRAMEBUFFER_REQUEST.get_response();
    let framebuffer = response.and_then(|resp| resp.framebuffers().next());

    let Some(fb) = framebuffer else {
        // Nothing we can draw on.
        return;
    };

    let fg = encode_color(&fb, Color::new(0xf4, 0xd3, 0x74));
    let bg = encode_color(&fb, Color::new(0x16, 0x1a, 0x1d));
    clear(&fb, bg);

    let text_width = message
        .split(|&b| b == b'\n')
        .map(|line| line.len() * (FONT_WIDTH + 1))
        .max()
        .unwrap_or(0);
    let lines = message.iter().filter(|&&b| b == b'\n').count() + 1;
    let text_height = lines * FONT_HEIGHT + (lines.saturating_sub(1)) * 2;

    let start_x = ((fb.width() as usize).saturating_sub(text_width)) / 2;
    let start_y = ((fb.height() as usize).saturating_sub(text_height)) / 2;

    render_text(&fb, message, start_x, start_y, fg, bg);
}

fn render_text(fb: &Framebuffer<'_>, text: &[u8], start_x: usize, mut y: usize, fg: u32, bg: u32) {
    let mut x = start_x;
    for &byte in text {
        if byte == b'\n' {
            x = start_x;
            y = y.saturating_add(FONT_HEIGHT + 2);
            continue;
        }
        draw_glyph(fb, byte, x, y, fg, bg);
        x = x.saturating_add(FONT_WIDTH + 1);
    }
}

/// PSF1 header: [0x36, 0x04, mode, charsize].
/// The spleen-8x16.psfu file is PSF1 with charsize=16 (0x10), mode=0x03.
/// Header is 4 bytes, glyph data starts at offset 4.
/// Each glyph is `charsize` bytes (one byte per row, 8 pixels wide, MSB-first).
fn draw_glyph(fb: &Framebuffer<'_>, byte: u8, x: usize, y: usize, fg: u32, bg: u32) {
    const PSF1_HEADER_SIZE: usize = 4;
    let charsize = FONT_8X16[3] as usize; // byte 3 = charsize in PSF1
    let glyph_offset = PSF1_HEADER_SIZE + (byte as usize) * charsize;
    let Some(glyph_data) = FONT_8X16.get(glyph_offset..glyph_offset + charsize) else {
        return;
    };

    for (row_idx, &row_bits) in glyph_data.iter().enumerate() {
        for col in 0..FONT_WIDTH {
            let color = if row_bits & (0x80 >> col) != 0 { fg } else { bg };
            put_pixel(fb, x + col, y + row_idx, color);
        }
    }
}

fn clear(fb: &Framebuffer<'_>, color: u32) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    for y in 0..height {
        for x in 0..width {
            put_pixel(fb, x, y, color);
        }
    }
}

fn put_pixel(fb: &Framebuffer<'_>, x: usize, y: usize, color: u32) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    if x >= width || y >= height {
        return;
    }

    let bytes_per_pixel = (fb.bpp() / 8) as usize;
    if bytes_per_pixel < 2 {
        return;
    }

    let pitch = fb.pitch() as usize;
    let base = fb.addr() as usize;
    let offset = y
        .saturating_mul(pitch)
        .saturating_add(x.saturating_mul(bytes_per_pixel));
    let pixel_ptr = (base + offset) as *mut u8;

    unsafe {
        match bytes_per_pixel {
            4 => {
                core::ptr::write_unaligned(pixel_ptr as *mut u32, color);
            }
            3 => {
                let bytes = color.to_le_bytes();
                core::ptr::copy_nonoverlapping(bytes.as_ptr(), pixel_ptr, 3);
            }
            2 => {
                let short = color as u16;
                core::ptr::write_unaligned(pixel_ptr as *mut u16, short);
            }
            _ => {}
        }
    }
}

#[derive(Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

fn encode_color(fb: &Framebuffer<'_>, color: Color) -> u32 {
    let r_mask = mask(fb.red_mask_size());
    let g_mask = mask(fb.green_mask_size());
    let b_mask = mask(fb.blue_mask_size());

    ((color.r as u32 & r_mask) << fb.red_mask_shift())
        | ((color.g as u32 & g_mask) << fb.green_mask_shift())
        | ((color.b as u32 & b_mask) << fb.blue_mask_shift())
}

const FONT_WIDTH: usize = 8;
const FONT_HEIGHT: usize = 16;

fn mask(bits: u8) -> u32 {
    if bits == 0 { 0 } else { (1u32 << bits) - 1 }
}

fn halt_forever() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}
