use std::{
    io::{Error, ErrorKind, Result, Write},
    path::Path,
};

use spidev::{Spidev, SpidevOptions};

static SOF: [u8; 1] = [0x72];

pub struct LedStrip {
    spi_dev: Spidev,
}

impl LedStrip {
    pub fn open<P: AsRef<Path>>(spi_dev: P) -> Result<Self> {
        let mut spi_dev = Spidev::open(spi_dev)?;
        let options = SpidevOptions::new().max_speed_hz(9_000_000).build();
        spi_dev.configure(&options)?;
        Ok(LedStrip { spi_dev })
    }

    pub fn send_image(&mut self, bytes: &[u8]) -> Result<()> {
        let written = self.spi_dev.write(&SOF)?;
        if written != SOF.len() {
            return Err(Error::new(ErrorKind::Interrupted, "Unable to write SOF"));
        }

        let written = self.spi_dev.write(bytes)?;
        if written != bytes.len() {
            return Err(Error::new(ErrorKind::Interrupted, "Unable to write bytes"));
        }

        Ok(())
    }
}
