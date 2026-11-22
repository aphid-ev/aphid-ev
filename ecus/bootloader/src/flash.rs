use crate::pac::FLASH;
use core::sync::atomic::{fence, Ordering};

const UNLOCK_KEY1: u32 = 0x45670123;
const UNLOCK_KEY2: u32 = 0xCDEF89AB;

/// The peripheral programs one double word at a time, and only at an address
/// aligned to one (RM0440, standard programming). The protocol's block
/// alignment is this granularity.
const PROGRAM_SIZE: usize = boot_protocol::PROGRAM_ALIGNMENT as usize;

/// Value a page reads as once erased. A trailing partial double word is
/// padded with it so the write leaves the tail indistinguishable from
/// untouched flash.
const ERASED_BYTE: u8 = 0xff;

pub struct FlashLayout {
    start_address: u32,
    page_size: u32,
    pub bootloader: MemoryRegion,
    pub application: MemoryRegion,
}

impl FlashLayout {
    pub const fn new(
        start_address: u32,
        memory_size: u32,
        page_size: u32,
        bootloader_size: u32,
    ) -> Self {
        assert!(start_address.is_multiple_of(page_size));
        assert!(memory_size > 0);
        assert!(memory_size.is_multiple_of(page_size));
        assert!(bootloader_size > 0);
        assert!(bootloader_size < memory_size);
        assert!(bootloader_size.is_multiple_of(page_size));

        // PNB in FLASH_CR is seven bits wide, so the peripheral cannot name a
        // page beyond 127. A layout that needs more has the wrong page size
        // for the part, and would silently erase the wrong page.
        assert!(memory_size / page_size <= 128);

        let bootloader = MemoryRegion {
            start_address,
            size: bootloader_size,
        };

        let application = MemoryRegion {
            start_address: start_address + bootloader_size,
            size: memory_size - bootloader_size,
        };

        Self {
            start_address,
            page_size,
            bootloader,
            application,
        }
    }

    pub fn application_pages(&self) -> PageIterator {
        PageIterator {
            current: ((self.application.start_address - self.start_address) / self.page_size) as u8,
            length: (self.application.size / self.page_size) as u8,
        }
    }
}

pub struct MemoryRegion {
    start_address: u32,
    size: u32,
}

impl MemoryRegion {
    pub fn absolute_address(&self, offset: u32) -> u32 {
        self.start_address + offset
    }

    pub fn vector_table(&self) -> u32 {
        self.start_address
    }

    pub fn reset_handler(&self) -> *const u32 {
        self.start_address as *const u32
    }
}

pub struct PageIterator {
    current: u8,
    length: u8,
}

impl Iterator for PageIterator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.length > 0 {
            self.length -= 1;
            let current = self.current;
            self.current += 1;
            Some(current)
        } else {
            None
        }
    }
}

pub enum Error {
    /// Target address is not aligned to a programmable double word.
    Alignment,
    /// The peripheral reported an erase or programming failure.
    Program,
}

pub type Result<T> = core::result::Result<T, Error>;

impl From<Error> for boot_protocol::Error {
    fn from(value: Error) -> Self {
        match value {
            Error::Alignment => Self::Alignment,
            Error::Program => Self::Program,
        }
    }
}

pub struct Flash {
    flash: FLASH,
}

impl Flash {
    pub fn new(flash: FLASH) -> Self {
        Self { flash }
    }

    pub fn set_latency(&mut self, latency: u8) {
        // Set FLASH latency before increasing sysclk
        self.flash
            .acr()
            .modify(|_, w| unsafe { w.latency().bits(latency) });
    }

    fn unlock(&mut self) {
        self.flash.keyr().write(|w| unsafe { w.bits(UNLOCK_KEY1) });
        self.flash.keyr().write(|w| unsafe { w.bits(UNLOCK_KEY2) });

        while self.flash.cr().read().lock().bit_is_set() {}
    }

    fn lock(&mut self) {
        self.flash.cr().modify(|_, w| w.lock().set_bit());
    }

    fn is_busy(&self) -> bool {
        self.flash.sr().read().bsy().bit_is_set()
    }

    /// The status flags are write-one-to-clear, so writing back whatever was
    /// read clears everything that is set. They must be cleared before an
    /// operation: the peripheral rejects one that starts with an error still
    /// pending, by raising PGSERR.
    fn clear_status(&mut self) {
        self.flash.sr().modify(|_, w| w);
    }

    /// Waits for the running operation and reports what it did. Clears the
    /// flags afterwards so the next operation starts from a clean slate.
    fn wait_ready(&mut self) -> Result<()> {
        while self.is_busy() {}

        let status = self.flash.sr().read();
        let failed = status.operr().bit_is_set()
            || status.progerr().bit_is_set()
            || status.wrperr().bit_is_set()
            || status.pgaerr().bit_is_set()
            || status.sizerr().bit_is_set()
            || status.pgserr().bit_is_set();

        self.clear_status();

        if failed {
            Err(Error::Program)
        } else {
            Ok(())
        }
    }

    /// Erases one page. `page` indexes the whole flash, and only the low
    /// seven bits reach the peripheral, so this addresses at most 128 pages.
    pub fn erase_page(&mut self, page: u8) -> Result<()> {
        while self.is_busy() {}

        self.clear_status();
        self.unlock();

        self.flash
            .cr()
            .modify(|_, w| unsafe { w.per().set_bit().pnb().bits(page) });
        self.flash.cr().modify(|_, w| w.strt().set_bit());

        let result = self.wait_ready();

        // PER has to go back down: left set, it makes the next programming
        // request fail as a sequence error.
        self.flash.cr().modify(|_, w| w.per().clear_bit());
        self.lock();

        result
    }

    pub fn write(&mut self, address: u32, data: &[u8]) -> Result<()> {
        if !address.is_multiple_of(PROGRAM_SIZE as u32) {
            return Err(Error::Alignment);
        }

        while self.is_busy() {}

        self.clear_status();
        self.unlock();

        // Programming is enabled once for the whole run. modify rather than
        // write, so the LOCK bit that unlock just cleared stays cleared.
        self.flash.cr().modify(|_, w| w.pg().set_bit());

        let mut result = Ok(());
        let mut target = address;

        for chunk in data.chunks(PROGRAM_SIZE) {
            let mut double_word = [ERASED_BYTE; PROGRAM_SIZE];
            double_word[..chunk.len()].copy_from_slice(chunk);

            result = self.program_double_word(target, &double_word);

            if result.is_err() {
                break;
            }

            target += PROGRAM_SIZE as u32;
        }

        self.flash.cr().modify(|_, w| w.pg().clear_bit());
        self.lock();

        result
    }

    fn program_double_word(&mut self, address: u32, data: &[u8; PROGRAM_SIZE]) -> Result<()> {
        let (words, []) = data.as_chunks::<4>() else {
            unreachable!("a double word is two words")
        };

        for (index, word) in words.iter().enumerate() {
            let word = u32::from_le_bytes(*word);

            // Both halves must reach the peripheral as two separate accesses
            // in order, or it reports a programming error.
            unsafe { (address as *mut u32).add(index).write_volatile(word) };
            fence(Ordering::SeqCst);
        }

        self.wait_ready()
    }
}
