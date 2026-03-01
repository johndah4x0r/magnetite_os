/*!
    Definitions specific to VESA/VBE operation on the PC platform
*/

// Framebuffer operation (WIP)
pub mod fb;

/**
    VGA/VESA screen information

    # Usage
    A constructor will not be provided, as this structure
    is intended to be instantiated across FFI boundaries.

    The instantiator of this structure must ensure that
    the fields contain valid values that reflect the
    current display mode.

    For instance, if `mode = 3` (80x25 16-color text mode),
    or otherwise corresponds to a VGA text mode, then
    the VESA-specific fields

    - `bytes_per_pixel`
    - `width`
    - `height`
    - `pitch`
    - `bits_per_pixel`
    - `packed_mask_low`
    - `packed_mask_high`
    - `packed_pos_low`
    - `packed_pos_high`
    - `frame_buf_low`
    - `frame_buf_high`

    should be set to zero, and the fields

    - `cells_x`
    - `cells_y`

    must contain the number of character columns and rows,
    respectively.

    If `mode > 0xff` (meaning that `mode` corresponds to a VESA
    graphics mode), or if `mode` corresponds to a VGA graphics mode,
    then the VESA-specific fields above must be filled with values
    obtained from the *VBE mode information block* (or otherwise
    derived from VGA-specific values). In addition, the fields

    - `cells_x`
    - `cells_y`

    must be set to zero, as the instantiatior cannot make any
    assumptions about the dimensions of a character cell, should
    the display be divided into character cells.
*/
#[repr(C)]
pub struct ScreenInfo {
    _mode: u16,
    _bytes_per_pixel: u16,
    _width: u16,
    _height: u16,
    _pitch: u16,
    _bits_per_pixel: u16,
    _cells_x: u16,
    _cells_y: u16,
    _packed_mask_low: u16,
    _packed_mask_high: u16,
    _packed_pos_low: u16,
    _packed_pos_high: u16,
    _frame_buf_low: u16,
    _frame_buf_high: u16,
}

impl ScreenInfo {
    /// Returns display mode
    pub fn mode(&self) -> usize {
        self._mode as usize
    }

    /**
        Returns number of bytes per pixel

        The returned value may be equal to zero if
        `mode()` corresponds to a VGA text mode.
    */
    pub fn bytes_per_pixel(&self) -> usize {
        self._bytes_per_pixel as usize
    }

    /**
        Returns display width in pixels

        The returned value may be equal to zero if
        `mode()` corresponds to a VGA text mode.
    */
    pub fn width(&self) -> usize {
        self._width as usize
    }

    /**
        Returns display height in pixels

        The returned value may be equal to zero if
        `mode()` corresponds to a VGA text mode,
    */
    pub fn height(&self) -> usize {
        self._height as usize
    }

    /**
        Returns display pitch (number of bytes per scanline)

        Note that this value may be greater than the product
        `width() * bytes_per_pixel()` if it is not aligned
        to a scanline boundary.

        The returned value may by equal to zero if
        `mode()` corresponds to a VGA text mode.
    */
    pub fn pitch(&self) -> usize {
        self._pitch as usize
    }

    /**
        Returns number of bits per pixel

        The returned value may by equal to zero if
        `mode()` corresponds to a VGA text mode.
    */
    pub fn bits_per_pixel(&self) -> usize {
        self._bits_per_pixel as usize
    }

    /**
        Returns number of character columns

        The returned value should be equal to zero if
        `mode()` does not correspond to a VGA text mode.
    */
    pub fn cells_x(&self) -> usize {
        self._cells_x as usize
    }

    /**
        Returns number of character columns

        The returned value should be equal to zero if
        `mode()` does not correspond to a VGA text mode.
    */
    pub fn cells_y(&self) -> usize {
        self._cells_y as usize
    }

    /**
        Returns packed mask sizes for the (X,R,G,B)
        channels

        The packed value is of the form `AA_RR_GG_BBh`. If one
        is concerned only about the (R,G,B) channels, then
        one can trivially obtain the packed value as follows:
        ```rust
        let rgb_mask: u32 = packed_mask() & 0x00_FF_FF_FF;

        /* example usage for 8:8:8 */
        let r_mask = (rgb_mask >> 16) & 0xFF;
        let g_mask = (rgb_mask >> 8) & 0xFF;
        let b_mask = (rgb_mask >> 0) & 0xFF;
        ```

        The packed value is provided mostly for convenience:
        ```rust
        match packed_mask() & 0x00_FF_FF_FF {
            0x00_05_06_05 => { /* 5:6:5 path */ },
            _ => { /* XRGB/ARGB path */ },
        }
        ```
    */
    pub fn packed_mask(&self) -> u32 {
        ((self._packed_mask_high as u32) << 16) | (self._packed_mask_low as u32)
    }

    /**
        Returns packed-encoded mask positions for the (X,R,G,B)
        channels

        The packed value is of the form `AA_RR_GG_BBh`. If one
        is concerned only about the (R,G,B) channels, then
        one can trivially obtain the packed value as follows:
        ```rust
        let rgb_pos: u32 = packed_pos() & 0x00_FF_FF_FF;

        /* example usage for 8:8:8 */
        let r_pos = (rgb_pos >> 16) & 0xFF;
        let g_pos = (rgb_pos >> 8) & 0xFF;
        let b_pos = (rgb_pos >> 0) & 0xFF;
        ```

        The packed value is provided mostly for convenience.
    */
    pub fn packed_pos(&self) -> u32 {
        ((self._packed_pos_high as u32) << 16) | (self._packed_pos_low as u32)
    }

    /**
        Returns pointer to the linear framebuffer

        The returned pointer should be equal to `None`
        if `mode()` corresponds to a VGA text mode.
    */
    pub fn frame_buf(&self) -> Option<*mut u8> {
        let addr = ((self._frame_buf_high as usize) << 16) | (self._frame_buf_low as usize);

        // I would have liked to use `NonNull<u8>` here, but
        // it requires compile-time guarantees that we simply
        // can't provide: only instantiator discipline and
        // runtime checking can help us (if at all)
        if addr == 0 {
            None
        } else {
            Some(addr as *mut u8)
        }
    }
}
