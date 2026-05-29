//! DMG PPU core timing and register model.
//!
//! This module models:
//! - LCD mode timing state machine (modes 0/1/2/3)
//! - LCD register behavior for `FF40..=FF4B`
//! - VRAM/OAM storage with CPU-access gating by active mode
//! - `VBlank` and LCD `STAT` interrupt event generation
//! - Frontend-agnostic frame buffer contract with frame-ready latching

const VRAM_LEN: usize = 0x2000;
const OAM_LEN: usize = 0xA0;

pub(crate) const FRAME_WIDTH: usize = 160;
pub(crate) const FRAME_HEIGHT: usize = 144;
pub(crate) const FRAME_PIXELS: usize = FRAME_WIDTH * FRAME_HEIGHT;

pub(crate) const VRAM_START: u16 = 0x8000;
pub(crate) const VRAM_END: u16 = 0x9FFF;
pub(crate) const OAM_START: u16 = 0xFE00;
pub(crate) const OAM_END: u16 = 0xFE9F;

pub(crate) const LCDC_ADDR: u16 = 0xFF40;
pub(crate) const STAT_ADDR: u16 = 0xFF41;
pub(crate) const SCY_ADDR: u16 = 0xFF42;
pub(crate) const SCX_ADDR: u16 = 0xFF43;
pub(crate) const LY_ADDR: u16 = 0xFF44;
pub(crate) const LYC_ADDR: u16 = 0xFF45;
pub(crate) const BGP_ADDR: u16 = 0xFF47;
pub(crate) const OBP0_ADDR: u16 = 0xFF48;
pub(crate) const OBP1_ADDR: u16 = 0xFF49;
pub(crate) const WY_ADDR: u16 = 0xFF4A;
pub(crate) const WX_ADDR: u16 = 0xFF4B;

const LCDC_ENABLE_BIT: u8 = 0b1000_0000;

const STAT_LYC_INT_ENABLE: u8 = 0b0100_0000;
const STAT_MODE2_INT_ENABLE: u8 = 0b0010_0000;
const STAT_MODE1_INT_ENABLE: u8 = 0b0001_0000;
const STAT_MODE0_INT_ENABLE: u8 = 0b0000_1000;
const STAT_SELECT_MASK: u8 = 0b0111_1000;
const STAT_COINCIDENCE_BIT: u8 = 0b0000_0100;

const MODE2_CYCLES: u16 = 80;
const MODE3_CYCLES: u16 = 172;
const MODE0_CYCLES: u16 = 204;
const SCANLINE_CYCLES: u16 = MODE2_CYCLES + MODE3_CYCLES + MODE0_CYCLES;
const LY_153_WRAP_DOTS: u16 = 4;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct PpuEvents {
    pub vblank_interrupt: bool,
    pub stat_interrupt: bool,
}

impl PpuEvents {
    fn merge(&mut self, other: Self) {
        self.vblank_interrupt |= other.vblank_interrupt;
        self.stat_interrupt |= other.stat_interrupt;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PpuMode {
    HBlank = 0,
    VBlank = 1,
    OamScan = 2,
    Drawing = 3,
}

#[derive(Debug)]
pub(crate) struct Ppu {
    vram: [u8; VRAM_LEN],
    oam: [u8; OAM_LEN],
    frame_buffer: [u8; FRAME_PIXELS],
    frame_ready: bool,
    lcdc: u8,
    stat_select: u8,
    scy: u8,
    scx: u8,
    ly: u8,
    lyc: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
    wy: u8,
    wx: u8,
    mode: PpuMode,
    dots_in_line: u16,
    ly_153_wrapped: bool,
    stat_irq_latch: bool,
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

impl Ppu {
    #[must_use]
    #[allow(clippy::large_stack_arrays)]
    pub(crate) fn new() -> Self {
        Self {
            vram: [0; VRAM_LEN],
            oam: [0; OAM_LEN],
            frame_buffer: [0; FRAME_PIXELS],
            frame_ready: false,
            lcdc: 0,
            stat_select: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0,
            obp0: 0,
            obp1: 0,
            wy: 0,
            wx: 0,
            mode: PpuMode::HBlank,
            dots_in_line: 0,
            ly_153_wrapped: false,
            stat_irq_latch: false,
        }
    }

    #[must_use]
    pub(crate) fn read_vram_cpu(&self, address: u16) -> u8 {
        if self.is_vram_blocked_for_cpu() {
            return 0xFF;
        }
        self.read_vram_direct(address)
    }

    pub(crate) fn write_vram_cpu(&mut self, address: u16, value: u8) {
        if self.is_vram_blocked_for_cpu() {
            return;
        }
        self.write_vram_direct(address, value);
    }

    #[must_use]
    pub(crate) fn read_oam_cpu(&self, address: u16) -> u8 {
        if self.is_oam_blocked_for_cpu() {
            return 0xFF;
        }
        self.read_oam_direct(address)
    }

    pub(crate) fn write_oam_cpu(&mut self, address: u16, value: u8) {
        if self.is_oam_blocked_for_cpu() {
            return;
        }
        self.write_oam_direct(address, value);
    }

    #[must_use]
    pub(crate) fn read_vram_direct(&self, address: u16) -> u8 {
        debug_assert!((VRAM_START..=VRAM_END).contains(&address));
        self.vram[usize::from(address - VRAM_START)]
    }

    #[must_use]
    pub(crate) fn read_oam_direct(&self, address: u16) -> u8 {
        debug_assert!((OAM_START..=OAM_END).contains(&address));
        self.oam[usize::from(address - OAM_START)]
    }

    pub(crate) fn write_oam_dma(&mut self, index: u8, value: u8) {
        let idx = usize::from(index);
        if idx < OAM_LEN {
            self.oam[idx] = value;
        }
    }

    #[must_use]
    pub(crate) fn read_io(&self, address: u16) -> u8 {
        match address {
            LCDC_ADDR => self.lcdc,
            STAT_ADDR => self.stat_value(),
            SCY_ADDR => self.scy,
            SCX_ADDR => self.scx,
            LY_ADDR => self.ly,
            LYC_ADDR => self.lyc,
            BGP_ADDR => self.bgp,
            OBP0_ADDR => self.obp0,
            OBP1_ADDR => self.obp1,
            WY_ADDR => self.wy,
            WX_ADDR => self.wx,
            _ => 0xFF,
        }
    }

    #[must_use]
    pub(crate) fn frame_buffer(&self) -> &[u8; FRAME_PIXELS] {
        &self.frame_buffer
    }

    #[must_use]
    pub(crate) fn take_frame_ready(&mut self) -> bool {
        let frame_ready = self.frame_ready;
        self.frame_ready = false;
        frame_ready
    }

    pub(crate) fn write_io(&mut self, address: u16, value: u8) -> PpuEvents {
        let mut events = PpuEvents::default();

        if address == LY_ADDR {
            // LY is read-only from the CPU perspective.
            return events;
        }

        match address {
            LCDC_ADDR => {
                let was_enabled = self.lcd_enabled();
                self.lcdc = value;
                let now_enabled = self.lcd_enabled();

                if !now_enabled {
                    self.ly = 0;
                    self.dots_in_line = 0;
                    self.mode = PpuMode::HBlank;
                    self.ly_153_wrapped = false;
                    self.frame_ready = false;
                } else if !was_enabled {
                    self.ly = 0;
                    self.dots_in_line = 0;
                    self.mode = PpuMode::OamScan;
                    self.ly_153_wrapped = false;
                    self.frame_ready = false;
                }

                if self.update_stat_irq_latch() {
                    events.stat_interrupt = true;
                }
            }
            STAT_ADDR => {
                self.stat_select = value & STAT_SELECT_MASK;
                if self.update_stat_irq_latch() {
                    events.stat_interrupt = true;
                }
            }
            SCY_ADDR => self.scy = value,
            SCX_ADDR => self.scx = value,
            LYC_ADDR => {
                self.lyc = value;
                if self.update_stat_irq_latch() {
                    events.stat_interrupt = true;
                }
            }
            BGP_ADDR => self.bgp = value,
            OBP0_ADDR => self.obp0 = value,
            OBP1_ADDR => self.obp1 = value,
            WY_ADDR => self.wy = value,
            WX_ADDR => self.wx = value,
            _ => {}
        }

        events
    }

    pub(crate) fn tick(&mut self, cycles: u8) -> PpuEvents {
        let mut events = PpuEvents::default();

        if !self.lcd_enabled() {
            return events;
        }

        let mut remaining = u16::from(cycles);
        while remaining > 0 {
            let until_transition = self.cycles_until_next_transition();
            let step = remaining.min(until_transition);
            self.dots_in_line += step;
            remaining -= step;

            if self.mode == PpuMode::VBlank
                && self.ly == 153
                && !self.ly_153_wrapped
                && self.dots_in_line == LY_153_WRAP_DOTS
            {
                self.ly = 0;
                self.ly_153_wrapped = true;
                if self.update_stat_irq_latch() {
                    events.stat_interrupt = true;
                }
            }

            if self.mode != PpuMode::VBlank && self.ly < 144 {
                if self.dots_in_line == MODE2_CYCLES {
                    self.mode = PpuMode::Drawing;
                    if self.update_stat_irq_latch() {
                        events.stat_interrupt = true;
                    }
                } else if self.dots_in_line == MODE2_CYCLES + MODE3_CYCLES {
                    self.mode = PpuMode::HBlank;
                    if self.update_stat_irq_latch() {
                        events.stat_interrupt = true;
                    }
                }
            }

            if self.dots_in_line == SCANLINE_CYCLES {
                self.dots_in_line = 0;
                if self.mode == PpuMode::VBlank && self.ly_153_wrapped {
                    self.mode = PpuMode::OamScan;
                    self.ly_153_wrapped = false;
                    if self.update_stat_irq_latch() {
                        events.stat_interrupt = true;
                    }
                } else {
                    events.merge(self.advance_scanline());
                }
            }
        }

        events
    }

    fn advance_scanline(&mut self) -> PpuEvents {
        let mut events = PpuEvents::default();

        self.ly = self.ly.wrapping_add(1);
        match self.ly.cmp(&144) {
            core::cmp::Ordering::Equal => {
                self.mode = PpuMode::VBlank;
                self.frame_ready = true;
                events.vblank_interrupt = true;
            }
            core::cmp::Ordering::Greater => {
                self.mode = PpuMode::VBlank;
                self.ly_153_wrapped = false;
            }
            core::cmp::Ordering::Less => {
                self.mode = PpuMode::OamScan;
            }
        }

        if self.update_stat_irq_latch() {
            events.stat_interrupt = true;
        }

        events
    }

    #[must_use]
    fn cycles_until_next_transition(&self) -> u16 {
        debug_assert!(self.lcd_enabled());

        if self.mode == PpuMode::VBlank && self.ly_153_wrapped {
            SCANLINE_CYCLES - self.dots_in_line
        } else if self.mode == PpuMode::VBlank
            && self.ly == 153
            && self.dots_in_line < LY_153_WRAP_DOTS
        {
            LY_153_WRAP_DOTS - self.dots_in_line
        } else if self.ly >= 144 {
            SCANLINE_CYCLES - self.dots_in_line
        } else if self.dots_in_line < MODE2_CYCLES {
            MODE2_CYCLES - self.dots_in_line
        } else if self.dots_in_line < MODE2_CYCLES + MODE3_CYCLES {
            MODE2_CYCLES + MODE3_CYCLES - self.dots_in_line
        } else {
            SCANLINE_CYCLES - self.dots_in_line
        }
    }

    #[must_use]
    fn stat_value(&self) -> u8 {
        let mode_bits = self.mode as u8;
        let coincidence = if self.ly == self.lyc {
            STAT_COINCIDENCE_BIT
        } else {
            0
        };
        0x80 | self.stat_select | coincidence | mode_bits
    }

    #[must_use]
    fn update_stat_irq_latch(&mut self) -> bool {
        let active = self.stat_irq_source_active();
        let rising_edge = active && !self.stat_irq_latch;
        self.stat_irq_latch = active;
        rising_edge
    }

    #[must_use]
    fn stat_irq_source_active(&self) -> bool {
        if !self.lcd_enabled() {
            return false;
        }

        let coincidence = (self.ly == self.lyc) && (self.stat_select & STAT_LYC_INT_ENABLE != 0);
        let mode0 = self.mode == PpuMode::HBlank && (self.stat_select & STAT_MODE0_INT_ENABLE != 0);
        let mode1 = self.mode == PpuMode::VBlank && (self.stat_select & STAT_MODE1_INT_ENABLE != 0);
        let mode2 =
            self.mode == PpuMode::OamScan && (self.stat_select & STAT_MODE2_INT_ENABLE != 0);

        coincidence || mode0 || mode1 || mode2
    }

    #[must_use]
    fn lcd_enabled(&self) -> bool {
        self.lcdc & LCDC_ENABLE_BIT != 0
    }

    #[must_use]
    fn is_vram_blocked_for_cpu(&self) -> bool {
        self.lcd_enabled() && self.mode == PpuMode::Drawing
    }

    #[must_use]
    fn is_oam_blocked_for_cpu(&self) -> bool {
        self.lcd_enabled() && (self.mode == PpuMode::OamScan || self.mode == PpuMode::Drawing)
    }

    fn write_vram_direct(&mut self, address: u16, value: u8) {
        debug_assert!((VRAM_START..=VRAM_END).contains(&address));
        self.vram[usize::from(address - VRAM_START)] = value;
    }

    fn write_oam_direct(&mut self, address: u16, value: u8) {
        debug_assert!((OAM_START..=OAM_END).contains(&address));
        self.oam[usize::from(address - OAM_START)] = value;
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[must_use]
pub(crate) fn decode_tile_row(low: u8, high: u8) -> [u8; 8] {
    let mut row = [0u8; 8];
    for (i, pixel) in row.iter_mut().enumerate() {
        let bit = 7 - i;
        let lo = (low >> bit) & 1;
        let hi = (high >> bit) & 1;
        *pixel = (hi << 1) | lo;
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode_bits(ppu: &Ppu) -> u8 {
        ppu.read_io(STAT_ADDR) & 0b11
    }

    #[test]
    fn test_mode_timing_transitions_on_visible_line() {
        let mut ppu = Ppu::new();
        ppu.write_io(LCDC_ADDR, 0x80);

        assert_eq!(mode_bits(&ppu), 2);

        ppu.tick(79);
        assert_eq!(mode_bits(&ppu), 2);

        ppu.tick(1);
        assert_eq!(mode_bits(&ppu), 3);

        ppu.tick(171);
        assert_eq!(mode_bits(&ppu), 3);

        ppu.tick(1);
        assert_eq!(mode_bits(&ppu), 0);

        ppu.tick(204);
        assert_eq!(ppu.read_io(LY_ADDR), 1);
        assert_eq!(mode_bits(&ppu), 2);
    }

    #[test]
    fn test_ly_progression_vblank_entry_and_wrap() {
        let mut ppu = Ppu::new();
        ppu.write_io(LCDC_ADDR, 0x80);

        for _ in 0..144 {
            ppu.tick(228);
            ppu.tick(228);
        }
        assert_eq!(ppu.read_io(LY_ADDR), 144);
        assert_eq!(mode_bits(&ppu), 1);

        for _ in 0..10 {
            ppu.tick(228);
            ppu.tick(228);
        }
        assert_eq!(ppu.read_io(LY_ADDR), 0);
        assert_eq!(mode_bits(&ppu), 2);
    }

    #[test]
    fn test_vblank_event_emitted_on_line_144_entry() {
        let mut ppu = Ppu::new();
        ppu.write_io(LCDC_ADDR, 0x80);

        let mut saw_vblank = false;
        for _ in 0..144 {
            let mut events = ppu.tick(228);
            events.merge(ppu.tick(228));
            if events.vblank_interrupt {
                saw_vblank = true;
            }
        }

        assert!(saw_vblank);
        assert_eq!(ppu.read_io(LY_ADDR), 144);
    }

    #[test]
    fn test_stat_write_masks_read_only_bits() {
        let mut ppu = Ppu::new();
        ppu.write_io(STAT_ADDR, 0xFF);

        let stat = ppu.read_io(STAT_ADDR);
        assert_eq!(stat & STAT_SELECT_MASK, STAT_SELECT_MASK);
        assert_eq!(stat & 0b11, 0);
        assert_eq!(stat & STAT_COINCIDENCE_BIT, STAT_COINCIDENCE_BIT);
    }

    #[test]
    fn test_stat_lyc_coincidence_and_edge_interrupt() {
        let mut ppu = Ppu::new();
        ppu.write_io(LCDC_ADDR, 0x80);
        let events = ppu.write_io(STAT_ADDR, STAT_LYC_INT_ENABLE);
        assert!(events.stat_interrupt);

        let no_repeat = ppu.tick(4);
        assert!(!no_repeat.stat_interrupt);

        let clear_match = ppu.write_io(LYC_ADDR, 1);
        assert!(!clear_match.stat_interrupt);

        let line_events = ppu.tick(228);
        let line_events_2 = ppu.tick(228);
        assert!(line_events.stat_interrupt || line_events_2.stat_interrupt);
    }

    #[test]
    fn test_lcdc_disable_resets_ly_and_mode() {
        let mut ppu = Ppu::new();
        ppu.write_io(LCDC_ADDR, 0x80);

        ppu.tick(228);
        ppu.tick(228);
        assert_eq!(ppu.read_io(LY_ADDR), 1);

        ppu.write_io(LCDC_ADDR, 0x00);
        assert_eq!(ppu.read_io(LY_ADDR), 0);
        assert_eq!(mode_bits(&ppu), 0);
    }

    #[test]
    fn test_lcd_disable_clears_ly153_wrap_phase() {
        let mut ppu = Ppu::new();
        ppu.write_io(LCDC_ADDR, 0x80);

        for _ in 0..153 {
            ppu.tick(228);
            ppu.tick(228);
        }
        assert_eq!(ppu.read_io(LY_ADDR), 153);

        ppu.tick(4);
        assert_eq!(ppu.read_io(LY_ADDR), 0);
        assert_eq!(mode_bits(&ppu), 1);

        ppu.write_io(LCDC_ADDR, 0x00);
        ppu.write_io(LCDC_ADDR, 0x80);

        assert_eq!(ppu.read_io(LY_ADDR), 0);
        assert_eq!(mode_bits(&ppu), 2);
    }

    #[test]
    fn test_lcd_off_halts_progression() {
        let mut ppu = Ppu::new();
        ppu.write_io(LCDC_ADDR, 0x00);
        ppu.tick(250);
        ppu.tick(250);

        assert_eq!(ppu.read_io(LY_ADDR), 0);
        assert_eq!(mode_bits(&ppu), 0);
    }

    #[test]
    fn test_vram_blocked_during_mode3_only() {
        let mut ppu = Ppu::new();
        ppu.write_io(LCDC_ADDR, 0x80);

        ppu.write_vram_cpu(VRAM_START, 0x12);
        assert_eq!(ppu.read_vram_cpu(VRAM_START), 0x12);

        ppu.tick(80);
        assert_eq!(mode_bits(&ppu), 3);
        assert_eq!(ppu.read_vram_cpu(VRAM_START), 0xFF);

        ppu.write_vram_cpu(VRAM_START, 0x34);
        ppu.tick(172);
        assert_eq!(mode_bits(&ppu), 0);
        assert_eq!(ppu.read_vram_cpu(VRAM_START), 0x12);
    }

    #[test]
    fn test_oam_blocked_during_modes2_and3() {
        let mut ppu = Ppu::new();
        ppu.write_io(LCDC_ADDR, 0x80);

        assert_eq!(mode_bits(&ppu), 2);
        ppu.write_oam_cpu(OAM_START, 0x55);
        assert_eq!(ppu.read_oam_cpu(OAM_START), 0xFF);

        ppu.tick(80);
        assert_eq!(mode_bits(&ppu), 3);
        ppu.write_oam_cpu(OAM_START, 0x66);
        assert_eq!(ppu.read_oam_cpu(OAM_START), 0xFF);

        ppu.tick(172);
        assert_eq!(mode_bits(&ppu), 0);
        ppu.write_oam_cpu(OAM_START, 0x77);
        assert_eq!(ppu.read_oam_cpu(OAM_START), 0x77);
    }

    #[test]
    fn test_ly_write_ignored_when_lcd_enabled() {
        let mut ppu = Ppu::new();
        ppu.write_io(LCDC_ADDR, 0x80);
        ppu.tick(228);
        ppu.tick(228);
        assert_eq!(ppu.read_io(LY_ADDR), 1);

        ppu.write_io(LY_ADDR, 0x99);
        assert_eq!(ppu.read_io(LY_ADDR), 1);
    }

    #[test]
    fn test_ly_write_ignored_when_lcd_disabled() {
        let mut ppu = Ppu::new();
        ppu.write_io(LY_ADDR, 0x99);
        assert_eq!(ppu.read_io(LY_ADDR), 0);
    }

    #[test]
    fn test_line_153_wraps_ly_to_zero_after_four_dots() {
        let mut ppu = Ppu::new();
        ppu.write_io(LCDC_ADDR, 0x80);

        for _ in 0..153 {
            ppu.tick(228);
            ppu.tick(228);
        }
        assert_eq!(ppu.read_io(LY_ADDR), 153);
        assert_eq!(mode_bits(&ppu), 1);

        ppu.tick(3);
        assert_eq!(ppu.read_io(LY_ADDR), 153);
        assert_eq!(mode_bits(&ppu), 1);

        ppu.tick(1);
        assert_eq!(ppu.read_io(LY_ADDR), 0);
        assert_eq!(mode_bits(&ppu), 1);

        ppu.tick(228);
        ppu.tick(224);
        assert_eq!(mode_bits(&ppu), 2);
    }

    #[test]
    fn test_decode_tile_row_bitplane_order() {
        assert_eq!(decode_tile_row(0x00, 0x00), [0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(decode_tile_row(0xFF, 0x00), [1, 1, 1, 1, 1, 1, 1, 1]);
        assert_eq!(decode_tile_row(0x00, 0xFF), [2, 2, 2, 2, 2, 2, 2, 2]);
        assert_eq!(decode_tile_row(0xFF, 0xFF), [3, 3, 3, 3, 3, 3, 3, 3]);

        let striped = decode_tile_row(0b1010_0101, 0b0101_1010);
        assert_eq!(striped, [1, 2, 1, 2, 2, 1, 2, 1]);
    }
}
