use crate::crc::Crc;
use crate::flash::{self, Flash, FlashLayout};
use crate::pac::SCB;
use boot_protocol::{Block, ImageError, SemVer, image};
use cortex_m::asm;

pub struct App {
    pub flash: &'static FlashLayout,
}

impl App {
    pub fn new(flash: &'static FlashLayout) -> Self {
        Self { flash }
    }

    /// Checks the image in the application region. Run it again after every
    /// update: the result describes the flash as it is now.
    pub fn validate(&self, crc: &mut Crc) -> Result<SemVer, ImageError> {
        image::validate(self.flash.application.as_slice(), crc)
    }

    /// Starts the application. Only call this with an image that `validate`
    /// has just accepted.
    pub fn bootload(self, scb: &mut SCB) -> ! {
        unsafe {
            // Relocate vector table
            scb.vtor.write(self.flash.application.vector_table());

            // Reset MSP and jump to Reset_Handler
            asm::bootload(self.flash.application.reset_handler())
        }
    }

    pub fn erase(&self, flash: &mut Flash) -> flash::Result<()> {
        for page in self.flash.application_pages() {
            flash.erase_page(page)?;
        }
        Ok(())
    }

    pub fn write<const N: usize>(&self, flash: &mut Flash, block: &Block<N>) -> flash::Result<()> {
        let address = self.flash.application.absolute_address(block.offset());

        flash.write(address, block.data())
    }
}
