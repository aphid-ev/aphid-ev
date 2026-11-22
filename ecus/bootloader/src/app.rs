use crate::flash::{self, Flash, FlashLayout};
use crate::pac::SCB;
use boot_protocol::{Block, SemVer};
use cortex_m::asm;

pub struct App {
    pub version: SemVer,
    pub flash: &'static FlashLayout,
}

impl App {
    pub fn new(flash: &'static FlashLayout) -> Self {
        // TODO: Read info from app
        let version = SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        };

        Self { version, flash }
    }

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
